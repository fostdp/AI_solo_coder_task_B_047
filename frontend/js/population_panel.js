class PopulationPanel {
    constructor() {
        this.currentSiteId = null;
        this.currentModel = 'allometric';
        this.populationData = null;
        this.onDataLoaded = null;
    }

    init() {
        document.querySelectorAll('.pop-model-btn').forEach(btn => {
            btn.addEventListener('click', (e) => {
                document.querySelectorAll('.pop-model-btn').forEach(b => b.classList.remove('active'));
                e.target.classList.add('active');
                this.currentModel = e.target.dataset.model;
                this.analyze();
            });
        });

        document.getElementById('refreshPopulation')?.addEventListener('click', () => {
            this.analyze();
        });
    }

    setSite(siteId) {
        this.currentSiteId = siteId;
        this.populationData = null;
        this.clearPanel();
        if (siteId) {
            this.analyze();
        }
    }

    clearPanel() {
        const panel = document.getElementById('populationInfo');
        if (panel) {
            panel.innerHTML = '<div class="loading">加载中...</div>';
        }
    }

    async analyze() {
        if (!this.currentSiteId) return;

        try {
            this.clearPanel();
            const data = await API.analyzePopulation(this.currentSiteId, this.currentModel);
            this.populationData = data;
            this.render(data);

            if (this.onDataLoaded) {
                this.onDataLoaded(data);
            }
        } catch (err) {
            console.error('人口分析失败:', err);
            const panel = document.getElementById('populationInfo');
            if (panel) {
                panel.innerHTML = `<div class="error">分析失败: ${err.message}</div>`;
            }
        }
    }

    render(data) {
        const panel = document.getElementById('populationInfo');
        if (!panel) return;

        const modelNames = {
            'allometric_growth': '异速生长模型',
            'residential_density': '居住密度模型',
            'idw_interpolation': '反距离加权插值'
        };

        const zonesHtml = data.zone_populations?.map(z => `
            <div class="pop-zone-item">
                <span class="pop-zone-name">${z.zone_type || '-'}</span>
                <span class="pop-zone-pop">${z.population?.toLocaleString() || '-'} 人</span>
                <div class="pop-zone-bar">
                    <div class="pop-zone-bar-fill" style="width: ${Math.min(100, (z.density / 50000) * 100)}%"></div>
                </div>
                <span class="pop-zone-density">${z.density?.toFixed(0) || '-'} 人/km²</span>
            </div>
        `).join('') || '';

        panel.innerHTML = `
            <div class="pop-summary">
                <div class="pop-stat">
                    <div class="pop-stat-value">${data.total_population?.toLocaleString() || '-'}</div>
                    <div class="pop-stat-label">估算总人口</div>
                </div>
                <div class="pop-stat">
                    <div class="pop-stat-value">${data.population_density_avg?.toFixed(0) || '-'}</div>
                    <div class="pop-stat-label">平均密度 (人/km²)</div>
                </div>
            </div>

            <div class="pop-model-info">
                <span class="label">模型：</span>
                <span class="value">${modelNames[data.model_type] || data.model_type || '-'}</span>
            </div>

            <div class="pop-zones-section">
                <h4>各功能区人口分布</h4>
                ${zonesHtml}
            </div>

            <div class="pop-confidence">
                <span class="label">置信度：</span>
                <span class="value">${(data.confidence * 100).toFixed(0)}%</span>
            </div>
        `;
    }
}
