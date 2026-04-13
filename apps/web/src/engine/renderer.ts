import { Color3 } from '@babylonjs/core/Maths/math.color';
import { StandardMaterial } from '@babylonjs/core/Materials/standardMaterial';
import type { InstancedMesh } from '@babylonjs/core/Meshes/instancedMesh';
import { MeshBuilder } from '@babylonjs/core/Meshes/meshBuilder';
import type { Scene } from '@babylonjs/core/scene';
import { Vector3 } from '@babylonjs/core/Maths/math.vector';
import type { Building, ZoneType } from '../types';

const MAX_INSTANCES = 2000;

type PoolType = 'res_l1' | 'res_l2' | 'res_l3' | 'com_l1' | 'com_l2' | 'com_l3' | 'ind_l1' | 'ind_l2' | 'ind_l3';

type PoolState = {
  instances: InstancedMesh[];
  active: boolean[];
};

const poolColor = (type: ZoneType): Color3 => {
  if (type === 'residential') {
    return new Color3(0.8, 0.72, 0.58);
  }
  if (type === 'commercial') {
    return new Color3(0.62, 0.78, 0.92);
  }
  return new Color3(0.55, 0.55, 0.58);
};

const poolKeyForBuilding = (building: Building): PoolType => {
  const prefix = building.type === 'residential' ? 'res' : building.type === 'commercial' ? 'com' : 'ind';
  return `${prefix}_l${building.level}` as PoolType;
};

export class RendererSystem {
  private readonly scene: Scene;

  private readonly pools = new Map<PoolType, PoolState>();

  private readonly assignments = new Map<string, { pool: PoolType; index: number }>();

  constructor(scene: Scene) {
    this.scene = scene;
  }

  async init(): Promise<void> {
    const poolTypes: PoolType[] = ['res_l1', 'res_l2', 'res_l3', 'com_l1', 'com_l2', 'com_l3', 'ind_l1', 'ind_l2', 'ind_l3'];
    for (const poolType of poolTypes) {
      const zoneType: ZoneType = poolType.startsWith('res') ? 'residential' : poolType.startsWith('com') ? 'commercial' : 'industrial';
      const base = MeshBuilder.CreateBox(`base-${poolType}`, { width: 7, depth: 7, height: 6 + Number(poolType.slice(-1)) * 3 }, this.scene);
      base.isVisible = false;
      const material = new StandardMaterial(`mat-${poolType}`, this.scene);
      material.diffuseColor = poolColor(zoneType);
      material.emissiveColor = Color3.Black();
      base.material = material;

      const instances: InstancedMesh[] = [];
      const active = new Array<boolean>(MAX_INSTANCES).fill(false);
      for (let i = 0; i < MAX_INSTANCES; i += 1) {
        const instance = base.createInstance(`${poolType}-${i}`);
        instance.scaling = Vector3.Zero();
        instance.position = new Vector3(0, -1000, 0);
        instance.receiveShadows = true;
        instances.push(instance);
      }

      this.pools.set(poolType, { instances, active });
    }
  }

  update(_dt: number): void {
    // Renderer reacts to grid events in v1.
  }

  spawnBuilding(building: Building, worldPosition: Vector3): void {
    const poolKey = poolKeyForBuilding(building);
    const pool = this.pools.get(poolKey);
    if (!pool) {
      return;
    }
    const index = pool.active.findIndex((isActive) => !isActive);
    if (index === -1) {
      return;
    }

    const instance = pool.instances[index];
    pool.active[index] = true;
    instance.position = worldPosition;
    instance.scaling = Vector3.One();
    this.assignments.set(building.id, { pool: poolKey, index });
  }

  demolishBuilding(buildingId: string): void {
    const assignment = this.assignments.get(buildingId);
    if (!assignment) {
      return;
    }
    const pool = this.pools.get(assignment.pool);
    if (!pool) {
      return;
    }
    const instance = pool.instances[assignment.index];
    instance.scaling = Vector3.Zero();
    instance.position.y = -1000;
    pool.active[assignment.index] = false;
    this.assignments.delete(buildingId);
  }
}

