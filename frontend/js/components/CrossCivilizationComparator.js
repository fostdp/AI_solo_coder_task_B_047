export class CrossCivilizationComparator {
    constructor(options = {}) {
        this.container = options.container || document.body;
        this.apiBase = options.apiBase || '/api';
        this.config = {
            normalizationMethod: options.normalizationMethod || 'minmax',
            enableClassification: options.enableClassification !== false,
            enableComparabilityAssessment: options.enableComparabilityAssessment !== false,
            compareByCategory: options.compareByCategory || 'all',
            chartType: options.chartType || 'radar',
            ...options.config,
        };
        this.currentResult = null;
        this.selectedCivilizations = options.selectedCivilizations || [];
    }

    async compare(civilizationIds = null) {
        if (civilizationIds) {
            this.selectedCivilizations = civilizationIds;
        }

        this._renderLoading();

        try {
            const url = `${this.apiBase}/civilization/compare`;
            const response = await fetch(url, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    civilization_ids: this.selectedCivilizations,
                    normalization_method: this.config.normalizationMethod,
                    compare_by_category: this.config.compareByCategory,
                    config: this.config,
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
            console.error('Civilization comparison failed:', error);
            this._renderError(error);
            throw error;
        }
    }

    async getCivilizations() {
        const response = await fetch(`${this.apiBase}/civilizations`);
        const result = await response.json();
        return result.data || result;
    }

    classifyCivilization(metrics) {
        return {
            size: this._classifySize(metrics.avg_area_sq_km),
            type: this._classifyType(metrics),
        };
    }

    renderRadarChart(canvasId) {
        if (!this.currentResult || !this.currentResult.radar_data) {
            console.warn('No radar data to render');
            return;
        }

        const canvas = document.getElementById(canvasId);
        if (!canvas) return;

        const ctx = canvas.getContext('2d');
        const { width, height } = canvas;
        const centerX = width / 2;
        const centerY = height / 2;
        const radius = Math.min(width, height) / 2 - 40;

        ctx.clearRect(0, 0, width, height);

        const indicators = this.currentResult.indicators || [];
        const numIndicators = indicators.length;

        for (let level = 1; level <= 5; level++) {
            const r = radius * (level / 5);
            ctx.strokeStyle = 'rgba(200, 200, 200, 0.5)';
            ctx.lineWidth = 1;
            ctx.beginPath();
            for (let i = 0; i <= numIndicators; i++) {
                const angle = (i / numIndicators) * Math.PI * 2 - Math.PI / 2;
                const x = centerX + r * Math.cos(angle);
                const y = centerY + r * Math.sin(angle);
                if (i === 0) ctx.moveTo(x, y);
                else ctx.lineTo(x, y);
            }
            ctx.stroke();
        }

        for (let i = 0; i < numIndicators; i++) {
            const angle = (i / numIndicators) * Math.PI * 2 - Math.PI / 2;
            const x = centerX + radius * Math.cos(angle);
            const y = centerY + radius * Math.sin(angle);

            ctx.strokeStyle = 'rgba(150, 150, 150, 0.5)';
            ctx.beginPath();
            ctx.moveTo(centerX, centerY);
            ctx.lineTo(x, y);
            ctx.stroke();

            ctx.fillStyle = '#333';
            ctx.font = '12px sans-serif';
            ctx.textAlign = 'center';
            const labelX = centerX + (radius + 20) * Math.cos(angle);
            const labelY = centerY + (radius + 20) * Math.sin(angle);
            ctx.fillText(indicators[i], labelX, labelY);
        }

        const colors = [
            'rgba(255, 99, 71, 0.3)',
            'rgba(65, 105, 225, 0.3)',
            'rgba(50, 205, 50, 0.3)',
            'rgba(255, 215, 0, 0.3)',
            'rgba(147, 112, 219, 0.3)',
        ];

        const borderColors = [
            'rgb(255, 99, 71)',
            'rgb(65, 105, 225)',
            'rgb(50, 205, 50)',
            'rgb(255, 215, 0)',
            'rgb(147, 112, 219)',
        ];

        this.currentResult.radar_data.forEach((data, civIndex) => {
            const values = data.values || [];
            ctx.fillStyle = colors[civIndex % colors.length];
            ctx.strokeStyle = borderColors[civIndex % borderColors.length];
            ctx.lineWidth = 2;
            ctx.beginPath();

            for (let i = 0; i <= numIndicators; i++) {
                const idx = i % numIndicators;
                const value = values[idx] || 0;
                const angle = (idx / numIndicators) * Math.PI * 2 - Math.PI / 2;
                const r = radius * value;
                const x = centerX + r * Math.cos(angle);
                const y = centerY + r * Math.sin(angle);
                if (i === 0) ctx.moveTo(x, y);
                else ctx.lineTo(x, y);
            }

            ctx.closePath();
            ctx.fill();
            ctx.stroke();
        });

        this._renderLegend(ctx, this.currentResult.radar_data, borderColors, 10, 10);
    }

    renderClassification(targetId) {
        if (!this.currentResult || !this.config.enableClassification) return;

        const target = document.getElementById(targetId);
        if (!target) return;

        const civs = this.currentResult.civilizations || [];
        const classifications = civs.map(civ => ({
            ...civ,
            classification: this._classifyFromSummary(civ),
        }));

        target.innerHTML = `
            <div class="classification-results">
                <h4>城市分类对比</h4>
                <table class="classification-table">
                    <thead>
                        <tr>
                            <th>文明</th>
                            <th>遗址数</th>
                            <th>规模分类</th>
                            <th>类型分类</th>
                            <th>置信度</th>
                        </tr>
                    </thead>
                    <tbody>
                        ${classifications.map(c => `
                            <tr>
                                <td>${c.name_cn || c.name}</td>
                                <td>${c.site_count}</td>
                                <td>${c.classification.size.label}</td>
                                <td>${c.classification.type.label}</td>
                                <td>${((c.classification.confidence || 0) * 100).toFixed(1)}%</td>
                            </tr>
                        `).join('')}
                    </tbody>
                </table>
            </div>
        `;
    }

    renderComparability(targetId) {
        if (!this.currentResult || !this.config.enableComparabilityAssessment) return;

        const target = document.getElementById(targetId);
        if (!target) return;

        const civNames = (this.currentResult.civilizations || [])
            .map(c => c.name_cn || c.name);

        const indicators = ['area', 'integration', 'choice', 'boundary_fd', 'road_fd',
            'compactness', 'road_density', 'functional_diversity'];

        const assessments = indicators.map(ind => ({
            indicator: ind,
            ...this._assessComparability(ind, civNames),
        }));

        target.innerHTML = `
            <div class="comparability-results">
                <h4>指标可比性评估</h4>
                <div class="comparability-list">
                    ${assessments.map(a => `
                        <div class="comparability-item ${a.comparable ? '' : 'warning'}">
                            <span class="indicator-name">${this._indicatorLabel(a.indicator)}</span>
                            <div class="confidence-bar">
                                <div class="confidence-fill" style="width: ${a.confidence * 100}%"></div>
                            </div>
                            <span class="confidence-value">${(a.confidence * 100).toFixed(0)}%</span>
                            ${a.adjustment_factor < 1.0 ? `<span class="adjustment">×${a.adjustment_factor.toFixed(2)}</span>` : ''}
                            ${!a.comparable ? '<span class="warning-icon">⚠️</span>' : ''}
                        </div>
                    `).join('')}
                </div>
                <p class="comparability-note">注：⚠️ 表示该指标可比性较低，建议谨慎解读</p>
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

    _classifySize(area) {
        if (area <= 1.0) return { label: '小型城市', confidence: 0.9, value: 'small' };
        if (area <= 5.0) return { label: '中型城市', confidence: 0.85, value: 'medium' };
        if (area <= 15.0) return { label: '大型城市', confidence: 0.8, value: 'large' };
        return { label: '超大型城市', confidence: 0.75, value: 'megalopolis' };
    }

    _classifyType(metrics) {
        const compactness = metrics.avg_compactness || 0.5;
        const diversity = metrics.avg_functional_diversity || 1.5;
        const roadDensity = metrics.avg_road_density || 5.0;
        const integration = metrics.avg_integration_global || 1.0;

        let bestType = '综合型';
        let bestScore = 0;

        if (compactness > 0.75 && integration > 1.2) {
            bestType = '都城';
            bestScore = 0.8;
        } else if (compactness < 0.5 && diversity < 1.2) {
            bestType = '祭祀中心';
            bestScore = 0.7;
        } else if (roadDensity > 8.0 && diversity > 1.8) {
            bestType = '商业城市';
            bestScore = 0.75;
        } else if (compactness > 0.5 && diversity > 1.5) {
            bestType = '地方城市';
            bestScore = 0.6;
        }

        return { label: bestType, confidence: bestScore, value: bestType };
    }

    _classifyFromSummary(civ) {
        return {
            size: { label: '待分类', confidence: 0.5, value: 'unknown' },
            type: { label: '待分类', confidence: 0.5, value: 'unknown' },
            confidence: 0.5,
        };
    }

    _assessComparability(indicator, civNames) {
        const notes = {
            area: '不同文明城市定义差异大，面积直接比较需谨慎',
            integration: '受城市形态影响，网格城市天然较高',
            choice: '受路网结构影响，放射状城市较高',
            boundary_fd: '受地形影响较大，平原城市更规整',
            road_fd: '受规划程度影响，规划城市分维低',
            compactness: '受城市定义影响，祭祀中心普遍偏低',
            road_density: '受人口密度和功能影响，可比性中等',
            functional_diversity: '受城市类型影响，都城功能更全',
        };

        const baseConfidence = {
            boundary_fd: 0.85, road_fd: 0.85,
            compactness: 0.75, integration: 0.75, choice: 0.75,
            road_density: 0.7, functional_diversity: 0.7,
            area: 0.5,
        };

        const base = baseConfidence[indicator] || 0.65;
        let adjustment = 1.0;

        if (indicator === 'area') {
            if (civNames.some(n => n.includes('玛雅'))) adjustment = 0.6;
            else if (civNames.some(n => n.includes('罗马'))) adjustment = 0.85;
        }

        return {
            comparable: base > 0.5,
            confidence: base * adjustment,
            notes: notes[indicator] || '可比性一般',
            adjustment_factor: adjustment,
        };
    }

    _indicatorLabel(key) {
        const labels = {
            area: '城市规模', integration: '整合度', choice: '选择度',
            boundary_fd: '边界分形维', road_fd: '路网分形维',
            compactness: '紧凑度', road_density: '道路密度',
            functional_diversity: '功能多样性',
        };
        return labels[key] || key;
    }

    _renderLegend(ctx, radarData, colors, x, y) {
        ctx.font = '12px sans-serif';
        radarData.forEach((data, i) => {
            ctx.fillStyle = colors[i % colors.length];
            ctx.fillRect(x, y + i * 20, 12, 12);
            ctx.fillStyle = '#333';
            ctx.fillText(data.civilization_name, x + 20, y + i * 20 + 10);
        });
    }

    _renderLoading() {
        this.container.innerHTML = `
            <div class="loading-indicator">
                <div class="spinner"></div>
                <p>跨文明比较中...</p>
            </div>
        `;
    }

    _renderError(error) {
        this.container.innerHTML = `
            <div class="error-message">
                <p>⚠️ 跨文明比较失败</p>
                <p class="error-detail">${error.message}</p>
            </div>
        `;
    }

    _renderResult() {
        this.container.innerHTML = `
            <div class="civilization-comparison-result">
                <h3>跨文明城市形态比较</h3>
                <div class="comparison-grid">
                    <div id="radar-chart-container">
                        <canvas id="comparison-radar-canvas" width="500" height="400"></canvas>
                    </div>
                    <div id="classification-container"></div>
                </div>
                <div id="comparability-container"></div>
            </div>
        `;

        setTimeout(() => {
            this.renderRadarChart('comparison-radar-canvas');
            this.renderClassification('classification-container');
            this.renderComparability('comparability-container');
        }, 0);
    }
}
