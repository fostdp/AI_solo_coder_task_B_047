export class PopulationReconstructor {
    constructor(options = {}) {
        this.container = options.container || document.body;
        this.apiBase = options.apiBase || '/api';
        this.config = {
            gridResolution: options.gridResolution || 16,
            interpolationMethod: options.interpolationMethod || 'kde',
            showHeatmap: options.showHeatmap !== false,
            showConfidence: options.showConfidence !== false,
            ...options.config,
        };
        this.currentResult = null;
        this.isLoading = false;
    }

    async analyze(siteId, params = {}) {
        this.isLoading = true;
        this._renderLoading();

        try {
            const url = `${this.apiBase}/population/${siteId}/analyze`;
            const response = await fetch(url, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    model_type: params.model_type || 'allometric_growth',
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
            console.error('Population reconstruction failed:', error);
            this._renderError(error);
            throw error;
        } finally {
            this.isLoading = false;
        }
    }

    async getDistribution(siteId) {
        const response = await fetch(`${this.apiBase}/population/${siteId}/distribution`);
        const result = await response.json();
        return result.data || result;
    }

    renderHeatmap(canvasId) {
        if (!this.currentResult || !this.currentResult.grid) {
            console.warn('No data to render heatmap');
            return;
        }

        const canvas = document.getElementById(canvasId);
        if (!canvas) {
            console.warn(`Canvas ${canvasId} not found`);
            return;
        }

        const ctx = canvas.getContext('2d');
        const { width, height } = canvas;
        ctx.clearRect(0, 0, width, height);

        const grid = this.currentResult.grid;
        const maxDensity = Math.max(...grid.map(g => g.density_km2));

        grid.forEach((point, index) => {
            const x = (index % this.config.gridResolution) / this.config.gridResolution * width;
            const y = Math.floor(index / this.config.gridResolution) / this.config.gridResolution * height;
            const intensity = point.density_km2 / maxDensity;

            ctx.fillStyle = this._getHeatmapColor(intensity);
            ctx.fillRect(x, y, width / this.config.gridResolution, height / this.config.gridResolution);
        });

        if (this.config.showConfidence) {
            this._renderConfidenceBadge(ctx, width, height);
        }
    }

    renderSummary(targetId) {
        if (!this.currentResult) return;

        const target = document.getElementById(targetId);
        if (!target) return;

        const r = this.currentResult;
        target.innerHTML = `
            <div class="population-summary">
                <div class="stat-card">
                    <span class="label">估计总人口</span>
                    <span class="value">${r.total_population?.toLocaleString() || 'N/A'}</span>
                </div>
                <div class="stat-card">
                    <span class="label">使用模型</span>
                    <span class="value">${r.method || 'N/A'}</span>
                </div>
                <div class="stat-card">
                    <span class="label">置信度</span>
                    <span class="value confidence-${Math.floor((r.confidence || 0) * 10)}">
                        ${((r.confidence || 0) * 100).toFixed(1)}%
                    </span>
                </div>
                <div class="stat-card">
                    <span class="label">数据质量</span>
                    <span class="value">${r.data_quality || 'N/A'}</span>
                </div>
            </div>
        `;
    }

    setConfig(config) {
        this.config = { ...this.config, ...config };
    }

    clear() {
        this.currentResult = null;
        this.container.innerHTML = '';
    }

    _getHeatmapColor(intensity) {
        const colors = [
            [0, 0, 255],
            [0, 255, 255],
            [0, 255, 0],
            [255, 255, 0],
            [255, 0, 0],
        ];
        const idx = Math.min(Math.floor(intensity * colors.length), colors.length - 1);
        const [r, g, b] = colors[idx];
        return `rgba(${r}, ${g}, ${b}, 0.8)`;
    }

    _renderConfidenceBadge(ctx, width, height) {
        const conf = this.currentResult.confidence || 0;
        ctx.fillStyle = 'rgba(0, 0, 0, 0.7)';
        ctx.fillRect(10, height - 40, 150, 30);
        ctx.fillStyle = 'white';
        ctx.font = '14px sans-serif';
        ctx.fillText(`置信度: ${(conf * 100).toFixed(1)}%`, 20, height - 18);
    }

    _renderLoading() {
        this.container.innerHTML = `
            <div class="loading-indicator">
                <div class="spinner"></div>
                <p>人口分布反演计算中...</p>
            </div>
        `;
    }

    _renderError(error) {
        this.container.innerHTML = `
            <div class="error-message">
                <p>⚠️ 人口分布反演失败</p>
                <p class="error-detail">${error.message}</p>
            </div>
        `;
    }

    _renderResult() {
        this.container.innerHTML = `
            <div class="population-result">
                <h3>人口分布反演结果</h3>
                <div id="population-heatmap-container"></div>
                <div id="population-summary"></div>
            </div>
        `;
    }
}
