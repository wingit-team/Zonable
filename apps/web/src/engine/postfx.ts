import '@babylonjs/core/PostProcesses/RenderPipeline/Pipelines/defaultRenderingPipeline';
import '@babylonjs/core/PostProcesses/RenderPipeline/Pipelines/ssao2RenderingPipeline';
import '@babylonjs/core/Rendering/prePassRendererSceneComponent';

import { DefaultRenderingPipeline } from '@babylonjs/core/PostProcesses/RenderPipeline/Pipelines/defaultRenderingPipeline';
import { SSAO2RenderingPipeline } from '@babylonjs/core/PostProcesses/RenderPipeline/Pipelines/ssao2RenderingPipeline';
import type { Scene } from '@babylonjs/core/scene';

export class PostFxSystem {
  private readonly scene: Scene;

  private ssao: SSAO2RenderingPipeline | null = null;

  private defaultPipeline: DefaultRenderingPipeline | null = null;

  constructor(scene: Scene) {
    this.scene = scene;
  }

  async init(): Promise<void> {
    const camera = this.scene.activeCamera;
    if (!camera) {
      return;
    }

    try {
      this.ssao = new SSAO2RenderingPipeline('ssao2', this.scene, {
        ssaoRatio: 1,
        blurRatio: 1
      });
      this.ssao.radius = 2;
      this.ssao.totalStrength = 1.2;
      this.ssao.samples = 16;
      this.scene.postProcessRenderPipelineManager.attachCamerasToRenderPipeline('ssao2', camera);
    } catch (error) {
      console.warn('[Zonable] SSAO pipeline unavailable, continuing without SSAO.', error);
      this.ssao = null;
    }

    this.defaultPipeline = new DefaultRenderingPipeline('default-pipeline', true, this.scene, [camera]);
    this.defaultPipeline.bloomEnabled = true;
    this.defaultPipeline.bloomThreshold = 0.8;
    this.defaultPipeline.bloomWeight = 0.3;
    this.defaultPipeline.bloomScale = 0.5;
    this.defaultPipeline.chromaticAberrationEnabled = true;
    this.defaultPipeline.chromaticAberration.aberrationAmount = 8;
    this.defaultPipeline.depthOfFieldEnabled = false;
    this.defaultPipeline.imageProcessingEnabled = true;
    this.defaultPipeline.imageProcessing.toneMappingEnabled = true;
    this.defaultPipeline.imageProcessing.toneMappingType = 0;
    this.defaultPipeline.imageProcessing.contrast = 1.1;
    this.defaultPipeline.imageProcessing.exposure = 1.0;
  }

  update(_dt: number): void {
    // No per-frame post process simulation yet.
  }

  setSSAOEnabled(enabled: boolean): void {
    if (!this.ssao) {
      return;
    }
    const camera = this.scene.activeCamera;
    if (!camera) {
      return;
    }
    if (enabled) {
      this.scene.postProcessRenderPipelineManager.attachCamerasToRenderPipeline('ssao2', camera);
      return;
    }
    this.scene.postProcessRenderPipelineManager.detachCamerasFromRenderPipeline('ssao2', camera);
  }

  setBloomEnabled(enabled: boolean): void {
    if (!this.defaultPipeline) {
      return;
    }
    this.defaultPipeline.bloomEnabled = enabled;
  }

  setShadowsEnabled(enabled: boolean): void {
    this.scene.shadowsEnabled = enabled;
  }

  setDofEnabled(enabled: boolean): void {
    if (!this.defaultPipeline) {
      return;
    }
    this.defaultPipeline.depthOfFieldEnabled = enabled;
  }
}
