import { Color3, Color4 } from '@babylonjs/core/Maths/math.color';
import { StandardMaterial } from '@babylonjs/core/Materials/standardMaterial';
import { VertexBuffer } from '@babylonjs/core/Buffers/buffer';
import type { AbstractMesh } from '@babylonjs/core/Meshes/abstractMesh';
import type { Mesh } from '@babylonjs/core/Meshes/mesh';
import { MeshBuilder } from '@babylonjs/core/Meshes/meshBuilder';
import { Vector3 } from '@babylonjs/core/Maths/math.vector';
import type { Scene } from '@babylonjs/core/scene';
import type { GridEventPayloadMap } from '../simulation/grid';

const GRID_SIZE = 150;
const TILE_SIZE = 10;

const ZONE_TINT: Record<string, Color3> = {
  none: new Color3(0.31, 0.38, 0.3),
  residential: Color3.FromHexString('#7ec87e'),
  commercial: Color3.FromHexString('#7eafc8'),
  industrial: Color3.FromHexString('#c8c07e')
};

const blendColor = (base: Color3, tint: Color3, mix = 0.4): Color4 =>
  new Color4(base.r * (1 - mix) + tint.r * mix, base.g * (1 - mix) + tint.g * mix, base.b * (1 - mix) + tint.b * mix, 1);

const tileCenter = (x: number, z: number): Vector3 =>
  new Vector3(x * TILE_SIZE - (GRID_SIZE * TILE_SIZE) / 2 + TILE_SIZE / 2, 0, z * TILE_SIZE - (GRID_SIZE * TILE_SIZE) / 2 + TILE_SIZE / 2);

export class TerrainSystem {
  private readonly scene: Scene;

  private ground: Mesh | null = null;

  private hoverOverlay: Mesh | null = null;

  private readonly roadPatches = new Map<string, Mesh>();

  private readonly serviceMarkers = new Map<string, Mesh>();

  private readonly baseColor = new Color3(0.23, 0.3, 0.22);

  constructor(scene: Scene) {
    this.scene = scene;
  }

  async init(): Promise<void> {
    this.ground = MeshBuilder.CreateGround(
      'terrain-ground',
      { width: GRID_SIZE * TILE_SIZE, height: GRID_SIZE * TILE_SIZE, subdivisions: GRID_SIZE },
      this.scene
    );
    this.ground.isPickable = true;

    const verticesCount = (this.ground.getTotalVertices() ?? 0);
    const colors: number[] = [];
    for (let i = 0; i < verticesCount; i += 1) {
      colors.push(this.baseColor.r, this.baseColor.g, this.baseColor.b, 1);
    }
    this.ground.setVerticesData(VertexBuffer.ColorKind, colors, true);
    const groundMaterial = new StandardMaterial('terrain-ground-material', this.scene);
    groundMaterial.specularColor = Color3.Black();
    groundMaterial.emissiveColor = new Color3(0.08, 0.1, 0.1);
    this.ground.material = groundMaterial;

    this.hoverOverlay = MeshBuilder.CreateGround('terrain-hover', { width: TILE_SIZE, height: TILE_SIZE }, this.scene);
    this.hoverOverlay.position.y = 0.03;
    this.hoverOverlay.isPickable = false;
    const hoverMaterial = new StandardMaterial('terrain-hover-material', this.scene);
    hoverMaterial.diffuseColor = new Color3(0.8, 0.9, 1);
    hoverMaterial.alpha = 0.28;
    this.hoverOverlay.material = hoverMaterial;
    this.hoverOverlay.setEnabled(false);

    this.scene.onPointerMove = () => {
      const pick = this.scene.pick(this.scene.pointerX, this.scene.pointerY, (mesh: AbstractMesh) => mesh.id === 'terrain-ground');
      if (!pick?.hit || !pick.pickedPoint || !this.hoverOverlay) {
        this.hoverOverlay?.setEnabled(false);
        return;
      }

      const x = Math.floor((pick.pickedPoint.x + (GRID_SIZE * TILE_SIZE) / 2) / TILE_SIZE);
      const z = Math.floor((pick.pickedPoint.z + (GRID_SIZE * TILE_SIZE) / 2) / TILE_SIZE);
      this.hoverOverlay.position.x = x * TILE_SIZE - (GRID_SIZE * TILE_SIZE) / 2 + TILE_SIZE / 2;
      this.hoverOverlay.position.z = z * TILE_SIZE - (GRID_SIZE * TILE_SIZE) / 2 + TILE_SIZE / 2;
      this.hoverOverlay.setEnabled(true);
    };
  }

  update(_dt: number): void {
    // Terrain is event-driven in v1.
  }

  onZoneChanged(payload: GridEventPayloadMap['zonable:grid:zone-changed']): void {
    if (!this.ground) {
      return;
    }
    const tint = ZONE_TINT[payload.zone] ?? ZONE_TINT.none;
    const colors = this.ground.getVerticesData(VertexBuffer.ColorKind);
    if (!colors) {
      return;
    }

    const [x, z] = payload.tileId.split('_').map(Number);
    const vertexIndex = z * (GRID_SIZE + 1) + x;
    const color = blendColor(this.baseColor, tint);
    const offset = vertexIndex * 4;
    colors[offset] = color.r;
    colors[offset + 1] = color.g;
    colors[offset + 2] = color.b;
    colors[offset + 3] = color.a;
    this.ground.updateVerticesData(VertexBuffer.ColorKind, colors);
  }

  onElevationChanged(payload: GridEventPayloadMap['zonable:grid:elevation-changed']): void {
    if (!this.ground) {
      return;
    }

    const positions = this.ground.getVerticesData(VertexBuffer.PositionKind);
    if (!positions) {
      return;
    }

    const [x, z] = payload.tileId.split('_').map(Number);
    const vertexIndex = z * (GRID_SIZE + 1) + x;
    positions[vertexIndex * 3 + 1] = payload.elevation;
    this.ground.updateVerticesData(VertexBuffer.PositionKind, positions);
  }

  onRoadChanged(payload: GridEventPayloadMap['zonable:grid:road-changed']): void {
    const [x, z] = payload.tileId.split('_').map(Number);
    const existing = this.roadPatches.get(payload.tileId);
    if (payload.road === 'none') {
      existing?.dispose();
      this.roadPatches.delete(payload.tileId);
      return;
    }
    if (existing) {
      return;
    }

    const patch = MeshBuilder.CreateGround(`road-${payload.tileId}`, { width: TILE_SIZE * 0.88, height: TILE_SIZE * 0.88 }, this.scene);
    patch.position = tileCenter(x, z);
    patch.position.y = 0.08;
    const mat = new StandardMaterial(`road-mat-${payload.tileId}`, this.scene);
    mat.diffuseColor = new Color3(0.18, 0.19, 0.2);
    mat.specularColor = Color3.Black();
    patch.material = mat;
    this.roadPatches.set(payload.tileId, patch);
  }

  upsertServiceMarker(tileId: string, service: string): void {
    const [x, z] = tileId.split('_').map(Number);
    const existing = this.serviceMarkers.get(tileId);
    if (existing) {
      existing.dispose();
    }

    const marker = MeshBuilder.CreateBox(`service-${tileId}`, { width: 3, depth: 3, height: 6 }, this.scene);
    marker.position = tileCenter(x, z).add(new Vector3(0, 3.2, 0));
    const mat = new StandardMaterial(`service-mat-${tileId}`, this.scene);
    mat.diffuseColor = service === 'fire' ? new Color3(0.9, 0.2, 0.2) : service === 'police' ? new Color3(0.2, 0.35, 0.9) : new Color3(0.25, 0.8, 0.85);
    marker.material = mat;
    this.serviceMarkers.set(tileId, marker);
  }

  removeServiceMarker(tileId: string): void {
    this.serviceMarkers.get(tileId)?.dispose();
    this.serviceMarkers.delete(tileId);
  }

  createRoadRibbon(path: Vector3[]): Mesh {
    return MeshBuilder.CreateRibbon(
      `road-ribbon-${Date.now()}`,
      {
        pathArray: [path.map((point) => point.add(new Vector3(0, 0.05, -0.3))), path.map((point) => point.add(new Vector3(0, 0.05, 0.3)))],
        closeArray: false,
        closePath: false,
        updatable: false
      },
      this.scene
    );
  }
}
