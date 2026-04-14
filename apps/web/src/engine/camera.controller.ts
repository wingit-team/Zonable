import { Vector3 } from '@babylonjs/core/Maths/math.vector';
import type { ArcRotateCamera } from '@babylonjs/core/Cameras/arcRotateCamera';

type CameraKey = 'forward' | 'backward' | 'left' | 'right' | 'rotateLeft' | 'rotateRight' | 'zoomIn' | 'zoomOut';

const DEFAULTS = {
  alpha: -Math.PI / 4,
  beta: Math.PI / 3.5,
  radius: 80,
  target: Vector3.Zero()
};

const KEY_MAP: Record<string, CameraKey> = {
  w: 'forward',
  s: 'backward',
  a: 'left',
  d: 'right',
  q: 'rotateLeft',
  e: 'rotateRight',
  r: 'zoomIn',
  f: 'zoomOut'
};

export class CameraController {
  private readonly camera: ArcRotateCamera;

  private readonly activeKeys = new Set<CameraKey>();

  private readonly onKeydown = (event: KeyboardEvent): void => {
    if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement || event.target instanceof HTMLSelectElement) {
      return;
    }
    const key = event.key.toLowerCase();
    if (key === 'h') {
      this.resetView();
      return;
    }
    const mapped = KEY_MAP[key];
    if (!mapped) {
      return;
    }
    this.activeKeys.add(mapped);
    event.preventDefault();
  };

  private readonly onKeyup = (event: KeyboardEvent): void => {
    const mapped = KEY_MAP[event.key.toLowerCase()];
    if (!mapped) {
      return;
    }
    this.activeKeys.delete(mapped);
  };

  constructor(camera: ArcRotateCamera) {
    this.camera = camera;
  }

  async init(): Promise<void> {
    window.addEventListener('keydown', this.onKeydown);
    window.addEventListener('keyup', this.onKeyup);

    window.addEventListener('blur', () => {
      this.activeKeys.clear();
    });
  }

  update(dt: number): void {
    const seconds = dt / 1000;
    const moveSpeed = 120 * seconds;
    const rotateSpeed = 1.2 * seconds;
    const zoomSpeed = 70 * seconds;

    const forward = new Vector3(Math.cos(this.camera.alpha), 0, Math.sin(this.camera.alpha)).normalize();
    const right = new Vector3(-forward.z, 0, forward.x);

    if (this.activeKeys.has('forward')) {
      this.camera.target.addInPlace(forward.scale(-moveSpeed));
    }
    if (this.activeKeys.has('backward')) {
      this.camera.target.addInPlace(forward.scale(moveSpeed));
    }
    if (this.activeKeys.has('left')) {
      this.camera.target.addInPlace(right.scale(-moveSpeed));
    }
    if (this.activeKeys.has('right')) {
      this.camera.target.addInPlace(right.scale(moveSpeed));
    }

    if (this.activeKeys.has('rotateLeft')) {
      this.camera.alpha += rotateSpeed;
    }
    if (this.activeKeys.has('rotateRight')) {
      this.camera.alpha -= rotateSpeed;
    }
    if (this.activeKeys.has('zoomIn')) {
      this.camera.radius -= zoomSpeed;
    }
    if (this.activeKeys.has('zoomOut')) {
      this.camera.radius += zoomSpeed;
    }

    this.camera.target.y = 0;
    this.clampState();
  }

  panToWorld(x: number, z: number): void {
    this.camera.setTarget(new Vector3(x, 0, z));
    this.clampState();
  }

  resetView(): void {
    this.camera.alpha = DEFAULTS.alpha;
    this.camera.beta = DEFAULTS.beta;
    this.camera.radius = DEFAULTS.radius;
    this.camera.setTarget(DEFAULTS.target.clone());
    this.clampState();
  }

  private clampState(): void {
    this.camera.beta = Math.min(Math.PI / 2.2, Math.max(0.3, this.camera.beta));
    this.camera.radius = Math.min(220, Math.max(15, this.camera.radius));
  }
}

