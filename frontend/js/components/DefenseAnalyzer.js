export class DefenseAnalyzer {
    constructor(options = {}) {
        this.container = options.container || document.body;
        this.apiBase = options.apiBase || '/api';
        this.config = {
            numSamplePoints: options.numSamplePoints || 36,
            showWeaponRange: options.showWeaponRange !== false,
            showAttackRoutes: options.showAttackRoutes !== false,
            asyncVisibility: options.asyncVisibility !== false,
            ...options.config,
        };
        this.currentResult = null;
        this.visibilityTask = null;
        this.isLoading = false;
    }

    async analyze(siteId, params = {}) {
        this.isLoading = true;
        this._renderLoading();

        try {
            const url = `${this.apiBase}/defense/${siteId}/analyze`;
            const response = await fetch(url, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    site_name: params.site_name || '',
                    wall_height_m: params.wall_height_m || 5.0,
                    wall_width_m: params.wall_width_m || 2.0,
                    moat_width_m: params.moat_width_m || 10.0,
                    terrain: params.terrain || 'plain',
                    async_visibility: this.config.asyncVisibility,
                    config: this.config,
                    ...params,
                }),
            });

            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }

            const result = await response.json();
            this.currentResult = result.data || result;
            this._renderResult();
            return this.currentResult;
        } catch (error) {
            console.error('Defense analysis failed:', error);
            this._renderError(error);
            throw error;
        } finally {
            this.isLoading = false;
        }
    }

    async computeVisibilityAsync(siteId, params = {}) {
        if (this.visibilityTask) {
            console.warn('Cancelling previous visibility task');
        }

        const controller = new AbortController();
        this.visibilityTask = controller;

        try {
            const url = `${this.apiBase}/defense/${siteId}/visibility`;
            const response = await fetch(url, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    async: true,
                    num_sample_points: this.config.numSamplePoints,
                    ...params,
                }),
                signal: controller.signal,
            });

            return await response.json();
        } finally {
            this.visibilityTask = null;
        }
    }

    cancelVisibility() {
        if (this.visibilityTask) {
            this.visibilityTask.abort();
            this.visibilityTask = null;
        }
    }

    renderVisibility(canvasId) {
        if (!this.currentResult || !this.currentResult.visibility) {
            console.warn('No visibility data to render');
            return;
        }

        const canvas = document.getElementById(canvasId);
        if (!canvas) return;

        const ctx = canvas.getContext('2d');
        const { width, height } = canvas;
        const centerX = width / 2;
        const centerY = height / 2;
        const radius = Math.min(width, height) / 2 - 20;

        ctx.clearRect(0, 0, width, height);

        const samplePoints = this.currentResult.visibility.sample_points || [];
        samplePoints.forEach((point) => {
            const angle = (point.angle_deg * Math.PI) / 180;
            const distRatio = Math.min(Math.max(point.distance_to_gate_km / 3.0, 0.1), 1.0);
            const r = radius * distRatio;
            const x = centerX + r * Math.cos(angle);
            const y = centerY + r * Math.sin(angle);

            const visibility = point.visibility || 0.5;
            ctx.fillStyle = this._getVisibilityColor(visibility);
            ctx.beginPath();
            ctx.arc(x, y, 5, 0, Math.PI * 2);
            ctx.fill();
        });

        if (this.config.showWeaponRange && this.currentResult.weapon_range) {
            this._renderWeaponRange(ctx, centerX, centerY, radius);
        }

        if (this.config.showAttackRoutes && this.currentResult.attack_routes) {
            this._renderAttackRoutes(ctx, centerX, centerY, radius);
        }
    }

    renderScore(targetId) {
        if (!this.currentResult) return;

        const target = document.getElementById(targetId);
        if (!target) return;

        const r = this.currentResult;
        const score = r.overall_score || 0;
        const weapon = r.weapon_range || {};

        target.innerHTML = `
            <div class="defense-score">
                <div class="score-circle" style="--score: ${score}">
                    <span class="score-value">${score.toFixed(1)}</span>
                    <span class="score-label">综合防御分</span>
                </div>
                <div class="defense-details">
                    <div class="detail-item">
                        <span class="label">武器系统</span>
                        <span class="value">${weapon.weapon_type || 'N/A'}</span>
                    </div>
                    <div class="detail-item">
                        <span class="label">典型射程</span>
                        <span class="value">${weapon.typical_range_m?.toFixed(0) || 'N/A'} m</span>
                    </div>
                    <div class="detail-item">
                        <span class="label">武器置信度</span>
                        <span class="value">${((weapon.confidence || 0) * 100).toFixed(1)}%</span>
                    </div>
                    <div class="detail-item">
                        <span class="label">武器覆盖</span>
                        <span class="value">${((r.visibility?.average_weapon_coverage || 0) * 100).toFixed(1)}%</span>
                    </div>
                    <div class="detail-item">
                        <span class="label">数据质量</span>
                        <span class="value">${((r.data_quality || 0) * 100).toFixed(1)}%</span>
                    </div>
                </div>
                ${weapon.literature_sources?.length ? `
                <div class="literature-sources">
                    <h5>文献来源</h5>
                    <ul>
                        ${weapon.literature_sources.map(s => `<li>${s}</li>`).join('')}
                    </ul>
                </div>
                ` : ''}
            </div>
        `;
    }

    setConfig(config) {
        this.config = { ...this.config, ...config };
    }

    clear() {
        this.cancelVisibility();
        this.currentResult = null;
        this.container.innerHTML = '';
    }

    _getVisibilityColor(visibility) {
        const hue = visibility * 120;
        return `hsl(${hue}, 80%, 50%)`;
    }

    _renderWeaponRange(ctx, centerX, centerY, radius) {
        const weapon = this.currentResult.weapon_range;
        const typicalKm = weapon.typical_range_m / 1000;
        const rangeRatio = Math.min(typicalKm / 3.0, 1.0);
        const rangeRadius = radius * rangeRatio;

        ctx.strokeStyle = 'rgba(255, 0, 0, 0.5)';
        ctx.lineWidth = 2;
        ctx.setLineDash([5, 5]);
        ctx.beginPath();
        ctx.arc(centerX, centerY, rangeRadius, 0, Math.PI * 2);
        ctx.stroke();
        ctx.setLineDash([]);

        ctx.fillStyle = 'rgba(255, 0, 0, 0.1)';
        ctx.beginPath();
        ctx.arc(centerX, centerY, rangeRadius, 0, Math.PI * 2);
        ctx.fill();
    }

    _renderAttackRoutes(ctx, centerX, centerY, radius) {
        const routes = this.currentResult.attack_routes || [];
        routes.forEach((route, index) => {
            const angle = (index / routes.length) * Math.PI * 2;
            const startX = centerX + radius * Math.cos(angle);
            const startY = centerY + radius * Math.sin(angle);

            ctx.strokeStyle = `rgba(255, 100, 0, ${0.3 + (route.attack_score || 0.5) * 0.7})`;
            ctx.lineWidth = 1 + (route.attack_score || 0.5) * 3;
            ctx.beginPath();
            ctx.moveTo(startX, startY);
            ctx.lineTo(centerX, centerY);
            ctx.stroke();
        });
    }

    _renderLoading() {
        this.container.innerHTML = `
            <div class="loading-indicator">
                <div class="spinner"></div>
                <p>防御效能分析中...</p>
                ${this.config.asyncVisibility ? '<p class="subtle">视线分析正在独立线程中运行</p>' : ''}
            </div>
        `;
    }

    _renderError(error) {
        this.container.innerHTML = `
            <div class="error-message">
                <p>⚠️ 防御效能分析失败</p>
                <p class="error-detail">${error.message}</p>
            </div>
        `;
    }

    _renderResult() {
        this.container.innerHTML = `
            <div class="defense-result">
                <h3>防御效能分析结果</h3>
                <div class="defense-grid">
                    <div id="defense-visibility-container">
                        <canvas id="defense-visibility-canvas" width="400" height="400"></canvas>
                    </div>
                    <div id="defense-score-container"></div>
                </div>
            </div>
        `;

        setTimeout(() => {
            this.renderVisibility('defense-visibility-canvas');
            this.renderScore('defense-score-container');
        }, 0);
    }
}
