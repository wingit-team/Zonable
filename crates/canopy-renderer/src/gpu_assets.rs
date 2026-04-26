//! GPU Asset Management with memory limits and LRU eviction.

use canopy_assets::handle::AssetId;
use canopy_assets::types::{Mesh, Texture};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use wgpu::{Buffer, Device, Queue};

pub const MAX_GPU_MEMORY_BYTES: usize = 2 * 1024 * 1024 * 1024; // 2 GB

/// Represents a mesh uploaded to the GPU.
pub struct GpuMesh {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub index_count: u32,
    pub index_u32: bool,
    pub memory_bytes: usize,
    pub last_used_frame: u64,
}

/// Represents a texture uploaded to the GPU.
pub struct GpuTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub memory_bytes: usize,
    pub last_used_frame: u64,
}

/// Manages GPU resources, enforcing a hard memory limit with LRU eviction.
pub struct GpuResourceManager {
    meshes: HashMap<AssetId, GpuMesh>,
    textures: HashMap<AssetId, GpuTexture>,
    
    current_memory_bytes: usize,
    current_frame: u64,
}

impl Default for GpuResourceManager {
    fn default() -> Self {
        Self {
            meshes: HashMap::new(),
            textures: HashMap::new(),
            current_memory_bytes: 0,
            current_frame: 0,
        }
    }
}

impl GpuResourceManager {
    pub fn begin_frame(&mut self) {
        self.current_frame += 1;
    }

    /// Mark an asset as used this frame so it is not evicted.
    pub fn mark_mesh_used(&mut self, id: AssetId) {
        if let Some(mesh) = self.meshes.get_mut(&id) {
            mesh.last_used_frame = self.current_frame;
        }
    }

    pub fn mark_texture_used(&mut self, id: AssetId) {
        if let Some(tex) = self.textures.get_mut(&id) {
            tex.last_used_frame = self.current_frame;
        }
    }

    pub fn get_mesh(&self, id: AssetId) -> Option<&GpuMesh> {
        self.meshes.get(&id)
    }

    pub fn get_texture(&self, id: AssetId) -> Option<&GpuTexture> {
        self.textures.get(&id)
    }

    /// Upload a mesh to the GPU. Evicts older resources if memory limit is reached.
    pub fn upload_mesh(&mut self, device: &Device, id: AssetId, mesh: &Mesh) {
        if self.meshes.contains_key(&id) {
            return;
        }

        let required_bytes = mesh.memory_bytes();
        self.ensure_memory_available(required_bytes);

        use wgpu::util::DeviceExt;
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Mesh_{:?}_VB", id)),
            contents: &mesh.vertices,
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Mesh_{:?}_IB", id)),
            contents: &mesh.indices,
            usage: wgpu::BufferUsages::INDEX,
        });

        self.current_memory_bytes += required_bytes;
        self.meshes.insert(id, GpuMesh {
            vertex_buffer,
            index_buffer,
            index_count: mesh.index_count,
            index_u32: mesh.index_u32,
            memory_bytes: required_bytes,
            last_used_frame: self.current_frame,
        });
        
        debug!("Uploaded mesh {:?} ({} bytes). Total GPU memory: {} MB", id, required_bytes, self.current_memory_bytes / 1024 / 1024);
    }

    /// Enforce the 2GB memory budget by evicting the least recently used resources.
    fn ensure_memory_available(&mut self, required_bytes: usize) {
        if self.current_memory_bytes + required_bytes <= MAX_GPU_MEMORY_BYTES {
            return;
        }

        warn!("GPU memory limit (2GB) reached. Evicting resources to free {} bytes...", required_bytes);

        // Collect all assets and their last used frame
        let mut candidates = Vec::new();
        for (id, mesh) in &self.meshes {
            candidates.push((*id, mesh.last_used_frame, mesh.memory_bytes, true)); // true = is_mesh
        }
        for (id, tex) in &self.textures {
            candidates.push((*id, tex.last_used_frame, tex.memory_bytes, false));
        }

        // Sort by least recently used (oldest frame first)
        candidates.sort_by_key(|c| c.1);

        for (id, _, bytes, is_mesh) in candidates {
            if self.current_memory_bytes + required_bytes <= MAX_GPU_MEMORY_BYTES {
                break; // Freed enough memory
            }

            if is_mesh {
                if self.meshes.remove(&id).is_some() {
                    self.current_memory_bytes -= bytes;
                    debug!("Evicted mesh {:?}", id);
                }
            } else {
                if self.textures.remove(&id).is_some() {
                    self.current_memory_bytes -= bytes;
                    debug!("Evicted texture {:?}", id);
                }
            }
        }

        if self.current_memory_bytes + required_bytes > MAX_GPU_MEMORY_BYTES {
            error!("Failed to free enough GPU memory! Budget exceeded.");
        }
    }
}
