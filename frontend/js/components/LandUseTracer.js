export class LandUseTracer {
    constructor(options = {}) {
        this.container = options.container || document.body;
        this.apiBase = options.apiBase || '/api';
        this.config = {
            enableStratigraphicInterpolation: options.enableStratigraphicInterpolation !== false,
            enableArchaeologicalValidation: options.enableArchaeologicalValidation !== false,
            fillGaps: options.fillGaps !== false,
            chartType: options.chartType || 'stacked_area',
            showCheckpoints: options.showCheckpoints !== false,
            terrain: options.terrain || 'plain',
            waterProximity: options.waterProximity || 0.3,
            ...options.config,
        };
        this.currentResult = null;
        this.checkpoints = options.checkpoints || [];
    }

    async analyze(siteId, params = {}) {
        this._renderLoading();

        try {
            const url = `${this.apiBase}/land_use/${siteId}/timeline`;
            const response = await fetch(url, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    fill_gaps: this.config.fillGaps,
                    stratigraphic_interpolation: this.config.enableStratigraphicInterpolation,
                    archaeological_validation: this.config.enableArchaeologicalValidation,
                    checkpoints: this.checkpoints,
                    terrain: this.config.terrain,
                    water_proximity: this.config.waterProximity,
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
            console.error('Land use trace failed:', error);
            this._renderError(error);
            throw error;
        }
    }

    async getTrend(siteId) {
        const response = await fetch(`${this.apiBase}/land_use/${siteId}/trend`);
        const result = await response.json();
        return result.data || result;
    }

    addCheckpoint(checkpoint) {
        this.checkpoints.push(checkpoint);
    }

    removeCheckpoint(periodYear) {
        this.checkpoints = this.checkpoints.filter(c => c.period_year !== periodYear);
    }

    renderTimelineChart(canvasId) {
        if (!this.currentResult || !this.currentResult.periods) {
            console.warn('No timeline data to render');
            return;
        }

        const canvas = document.getElementById(canvasId);
        if (!canvas) return;

        const ctx = canvas.getContext('2d');
        const { width, height } = canvas;
        const padding = { top: 40, right: 80, bottom: 60, left: 60 };
        const chartWidth = width - padding.left - padding.right;
        const chartHeight = height - padding.top - padding.bottom;

        ctx.clearRect(0, 0, width, height);

        const periods = this.currentResult.periods || [];
        const landUseTypes = this.currentResult.land_use_types || [
            'urban', 'farmland', 'forest', 'grassland', 'wetland', 'water', 'wasteland', 'settlement',
        ];

        const colors = {
            urban: '#e74c3c', farmland: '#f39c12', forest: '#27ae60',
            grassland: '#1abc9c', wetland: '#3498db', water: '#2980b9',
            wasteland: '#95a5a6', settlement: '#e67e22',
        };

        const labels = {
            urban: '城市', farmland: '农田', forest: '森林',
            grassland: '草地', wetland: '湿地', water: '水体',
            wasteland: '荒地', settlement: '居民点',
        };

        const xScale = (i) => padding.left + (i / (periods.length - 1 || 1)) * chartWidth;
        const yScale = (v) => padding.top + chartHeight - (v / 100) * chartHeight;

        let cumulative = periods.map(() => 0);

        landUseTypes.forEach(type => {
            const values = periods.map(p => {
                const item = p.land_uses?.find(u => u.land_use_type === type);
                return item?.percentage || 0;
            });

            ctx.fillStyle = colors[type] || '#999';
            ctx.beginPath();
            ctx.moveTo(padding.left, yScale(cumulative[0]));

            for (let i = 0; i < periods.length; i++) {
                const y = yScale(cumulative[i] + values[i]);
                cumulative[i] += values[i];
                ctx.lineTo(xScale(i), y);
            }

            for (let i = periods.length - 1; i >= 0; i--) {
                ctx.lineTo(xScale(i), yScale(cumulative[i] - values[i]));
            }

            ctx.closePath();
            ctx.fill();

            ctx.strokeStyle = 'rgba(255, 255, 255, 0.3)';
            ctx.lineWidth = 1;
            ctx.beginPath();
            for (let i = 0; i < periods.length; i++) {
                const y = yScale(cumulative[i]);
                if (i === 0) ctx.moveTo(xScale(i), y);
                else ctx.lineTo(xScale(i), y);
            }
            ctx.stroke();
        });

        ctx.strokeStyle = '#ccc';
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(padding.left, padding.top);
        ctx.lineTo(padding.left, height - padding.bottom);
        ctx.lineTo(width - padding.right, height - padding.bottom);
        ctx.stroke();

        ctx.fillStyle = '#333';
        ctx.font = '11px sans-serif';
        ctx.textAlign = 'center';
        periods.forEach((p, i) => {
            ctx.fillText(p.period_name || p.period_year, xScale(i), height - padding.bottom + 20);
        });

        ctx.textAlign = 'right';
        for (let p = 0; p <= 100; p += 20) {
            ctx.fillText(`${p}%`, padding.left - 10, yScale(p) + 4);
            ctx.strokeStyle = 'rgba(200, 200, 200, 0.3)';
            ctx.beginPath();
            ctx.moveTo(padding.left, yScale(p));
            ctx.lineTo(width - padding.right, yScale(p));
            ctx.stroke();
        }

        this._renderLandUseLegend(ctx, landUseTypes, colors, labels, width - padding.right + 10, padding.top);

        if (this.config.showCheckpoints && this.checkpoints.length > 0) {
            this._renderCheckpoints(ctx, periods, xScale, yScale, padding);
        }
    }

    renderTrendSummary(targetId) {
        if (!this.currentResult) return;

        const target = document.getElementById(targetId);
        if (!target) return;

        const periods = this.currentResult.periods || [];
        const first = periods[0]?.land_uses || [];
        const last = periods[periods.length - 1]?.land_uses || [];

        const getPct = (items, type) => items.find(u => u.land_use_type === type)?.percentage || 0;

        const urbanDecay = getPct(first, 'urban') > 0
            ? ((getPct(first, 'urban') - getPct(last, 'urban')) / getPct(first, 'urban')) * 100
            : 0;

        const farmlandGrowth = getPct(first, 'farmland') > 0
            ? ((getPct(last, 'farmland') - getPct(first, 'farmland')) / getPct(first, 'farmland')) * 100
            : 0;

        target.innerHTML = `
            <div class="trend-summary">
                <h4>土地利用变迁趋势</h4>
                <div class="trend-cards">
                    <div class="trend-card decline">
                        <span class="trend-label">城市用地衰减</span>
                        <span class="trend-value">${urbanDecay.toFixed(1)}%</span>
                    </div>
                    <div class="trend-card growth">
                        <span class="trend-label">农田扩张</span>
                        <span class="trend-value">${farmlandGrowth.toFixed(1)}%</span>
                    </div>
                    <div class="trend-card">
                        <span class="trend-label">时期数</span>
                        <span class="trend-value">${periods.length}</span>
                    </div>
                    <div class="trend-card">
                        <span class="trend-label">土地类型</span>
                        <span class="trend-value">${this.currentResult.land_use_types?.length || 8}</span>
                    </div>
                </div>
                ${this.checkpoints.length > 0 ? `
                <div class="checkpoints-info">
                    <p>🏛️ 考古校验点：${this.checkpoints.length} 个</p>
                </div>
                ` : ''}
                ${this.config.enableStratigraphicInterpolation ? `
                <div class="method-info">
                    <p>📊 地层插值：已启用</p>
                </div>
                ` : ''}
            </div>
        `;
    }

    renderStratigraphicInfo(targetId) {
        if (!this.currentResult || !this.currentResult.stratigraphy) return;

        const target = document.getElementById(targetId);
        if (!target) return;

        const s = this.currentResult.stratigraphy;

        target.innerHTML = `
            <div class="stratigraphic-info">
                <h4>地层学信息</h4>
                <div class="stratigraphy-data">
                    <div class="info-item">
                        <span class="label">连续性指数</span>
                        <span class="value">${((s.continuity || 0) * 100).toFixed(1)}%</span>
                    </div>
                    <div class="info-item">
                        <span class="label">沉积模型</span>
                        <span class="value">${s.deposition_model || 'alluvial_plain'}</span>
                    </div>
                    <div class="info-item">
                        <span class="label">堆积速率</span>
                        <span class="value">${s.accumulation_rate?.toFixed(3) || 'N/A'} cm/yr</span>
                    </div>
                </div>
                ${s.gaps?.length > 0 ? `
                <div class="gaps-warning">
                    <p>⚠️ 检测到 ${s.gaps.length} 处地层间断：</p>
                    <ul>
                        ${s.gaps.map(g => `<li>${g}</li>`).join('')}
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
        this.currentResult = null;
        this.checkpoints = [];
        this.container.innerHTML = '';
    }

    _renderLandUseLegend(ctx, types, colors, labels, x, y) {
        ctx.font = '11px sans-serif';
        types.forEach((type, i) => {
            ctx.fillStyle = colors[type] || '#999';
            ctx.fillRect(x, y + i * 18, 12, 12);
            ctx.fillStyle = '#333';
            ctx.fillText(labels[type] || type, x + 20, y + i * 18 + 10);
        });
    }

    _renderCheckpoints(ctx, periods, xScale, yScale, padding) {
        ctx.fillStyle = 'rgba(255, 215, 0, 0.8)';
        ctx.strokeStyle = '#b8860b';
        ctx.lineWidth = 2;

        this.checkpoints.forEach(cp => {
            const periodIndex = periods.findIndex(p => p.period_year === cp.period_year);
            if (periodIndex >= 0) {
                const x = xScale(periodIndex);
                const y = padding.top - 10;

                ctx.beginPath();
                ctx.moveTo(x, y);
                ctx.lineTo(x - 8, y - 12);
                ctx.lineTo(x + 8, y - 12);
                ctx.closePath();
                ctx.fill();
                ctx.stroke();

                ctx.fillStyle = '#333';
                ctx.font = '10px sans-serif';
                ctx.textAlign = 'center';
                ctx.fillText('🏛️', x, y - 15);
            }
        });
    }

    _renderLoading() {
        this.container.innerHTML = `
            <div class="loading-indicator">
                <div class="spinner"></div>
                <p>土地利用变迁分析中...</p>
                ${this.config.enableStratigraphicInterpolation ? '<p class="subtle">地层插值已启用</p>' : ''}
            </div>
        `;
    }

    _renderError(error) {
        this.container.innerHTML = `
            <div class="error-message">
                <p>⚠️ 土地利用变迁分析失败</p>
                <p class="error-detail">${error.message}</p>
            </div>
        `;
    }

    _renderResult() {
        this.container.innerHTML = `
            <div class="land-use-result">
                <h3>土地利用变迁分析结果</h3>
                <div class="land-use-grid">
                    <div id="timeline-chart-container">
                        <canvas id="land-use-timeline-canvas" width="700" height="350"></canvas>
                    </div>
                    <div id="trend-summary-container"></div>
                </div>
                <div id="stratigraphic-info-container"></div>
            </div>
        `;

        setTimeout(() => {
            this.renderTimelineChart('land-use-timeline-canvas');
            this.renderTrendSummary('trend-summary-container');
            this.renderStratigraphicInfo('stratigraphic-info-container');
        }, 0);
    }
}
