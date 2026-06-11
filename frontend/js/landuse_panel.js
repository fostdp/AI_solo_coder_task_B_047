class LandUsePanel {
    constructor() {
        this.currentSiteId = null;
        this.landUseData = null;
        this.canvas = null;
        this.ctx = null;
    }

    init() {
        this.canvas = document.getElementById('landuseChart');
        if (this.canvas) {
            this.ctx = this.canvas.getContext('2d');
        }
    }

    setSite(siteId) {
        this.currentSiteId = siteId;
        this.landUseData = null;
        this.clearPanel();
        if (siteId) {
            this.loadData();
        }
    }

    clearPanel() {
        const info = document.getElementById('landuseInfo');
        if (info) {
            info.innerHTML = '<div class="loading">加载中...</div>';
        }
        if (this.ctx) {
            this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
        }
    }

    async loadData() {
        if (!this.currentSiteId) return;

        try {
            const data = await API.getLandUseTimeline(this.currentSiteId);
            this.landUseData = data;
            this.render(data);
        } catch (err) {
            console.error('土地利用数据加载失败:', err);
            const info = document.getElementById('landuseInfo');
            if (info) {
                info.innerHTML = `<div class="error">加载失败: ${err.message}</div>`;
            }
        }
    }

    render(data) {
        const info = document.getElementById('landuseInfo');
        if (info && data.periods) {
            info.innerHTML = `
                <div class="landuse-summary">
                    <div class="landuse-summary-item">
                        <span class="label">时期数：</span>
                        <span class="value">${data.periods.length} 个</span>
                    </div>
                    <div class="landuse-summary-item">
                        <span class="label">时间跨度：</span>
                        <span class="value">${data.periods[0]?.period_name || ''} - ${data.periods[data.periods.length - 1]?.period_name || ''}</span>
                    </div>
                </div>
                <div class="landuse-legend">
                    ${Object.entries(CONFIG.LAND_USE_LABELS).map(([key, label]) => `
                        <div class="landuse-legend-item">
                            <div class="landuse-legend-color" style="background: ${CONFIG.LAND_USE_COLORS[key]}"></div>
                            <span>${label}</span>
                        </div>
                    `).join('')}
                </div>
            `;
        }

        if (this.ctx && data.periods && data.periods.length > 0) {
            this.renderStackedAreaChart(data.periods);
        }
    }

    renderStackedAreaChart(periods) {
        const canvas = this.canvas;
        const ctx = this.ctx;
        const W = canvas.width;
        const H = canvas.height;
        const padding = { top: 20, right: 20, bottom: 40, left: 50 };
        const chartW = W - padding.left - padding.right;
        const chartH = H - padding.top - padding.bottom;

        ctx.clearRect(0, 0, W, H);

        const landUseTypes = ['urban', 'farmland', 'forest', 'grassland', 'wetland', 'water', 'wasteland', 'settlement'];

        const dataPoints = periods.length;
        const xStep = chartW / (dataPoints - 1);

        let maxTotal = 0;
        periods.forEach(p => {
            const total = p.land_uses?.reduce((sum, lu) => sum + (lu.percentage || 0), 0) || 0;
            if (total > maxTotal) maxTotal = total;
        });
        if (maxTotal < 100) maxTotal = 100;

        const yScale = chartH / maxTotal;

        const yTicks = [0, 25, 50, 75, 100];
        ctx.strokeStyle = '#ecf0f1';
        ctx.fillStyle = '#7f8c8d';
        ctx.font = '10px sans-serif';
        ctx.textAlign = 'right';
        ctx.textBaseline = 'middle';

        yTicks.forEach(t => {
            const y = padding.top + chartH - t * yScale;
            ctx.beginPath();
            ctx.moveTo(padding.left, y);
            ctx.lineTo(padding.left + chartW, y);
            ctx.stroke();
            ctx.fillText(t + '%', padding.left - 5, y);
        });

        const cumulativeBottoms = new Array(dataPoints).fill(0);

        landUseTypes.forEach(type => {
            const typeData = periods.map((p, i) => {
                const lu = p.land_uses?.find(l => l.land_use_type === type);
                return { x: padding.left + i * xStep, y: lu?.percentage || 0 };
            });

            ctx.fillStyle = CONFIG.LAND_USE_COLORS[type] || CONFIG.LAND_USE_COLORS.default;
            ctx.globalAlpha = 0.8;
            ctx.beginPath();
            ctx.moveTo(typeData[0].x, padding.top + chartH - cumulativeBottoms[0] * yScale);

            typeData.forEach((d, i) => {
                const yTop = padding.top + chartH - (cumulativeBottoms[i] + d.y) * yScale;
                ctx.lineTo(d.x, yTop);
            });

            for (let i = typeData.length - 1; i >= 0; i--) {
                const yBottom = padding.top + chartH - cumulativeBottoms[i] * yScale;
                ctx.lineTo(typeData[i].x, yBottom);
            }

            ctx.closePath();
            ctx.fill();

            typeData.forEach((d, i) => {
                cumulativeBottoms[i] += d.y;
            });
        });

        ctx.globalAlpha = 1;

        ctx.fillStyle = '#2c3e50';
        ctx.font = '10px sans-serif';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'top';

        periods.forEach((p, i) => {
            const x = padding.left + i * xStep;
            ctx.save();
            ctx.translate(x, padding.top + chartH + 5);
            ctx.rotate(-Math.PI / 6);
            ctx.fillText(p.period_name || '', 0, 0);
            ctx.restore();
        });

        ctx.strokeStyle = '#bdc3c7';
        ctx.lineWidth = 1;
        ctx.strokeRect(padding.left, padding.top, chartW, chartH);
    }

    resize() {
        if (this.canvas && this.landUseData?.periods) {
            const rect = this.canvas.parentElement.getBoundingClientRect();
            this.canvas.width = rect.width || 400;
            this.canvas.height = 200;
            this.renderStackedAreaChart(this.landUseData.periods);
        }
    }
}
