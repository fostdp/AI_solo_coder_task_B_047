class CivilizationCompare {
    constructor() {
        this.civilizations = [];
        this.selectedIds = [];
        this.comparisonData = null;
        this.radarCanvas = null;
        this.radarCtx = null;
        this.isOpen = false;
    }

    init() {
        this.radarCanvas = document.getElementById('civRadarChart');
        if (this.radarCanvas) {
            this.radarCtx = this.radarCanvas.getContext('2d');
        }

        document.getElementById('civCompareBtn')?.addEventListener('click', () => {
            this.open();
        });

        document.getElementById('civModalClose')?.addEventListener('click', () => {
            this.close();
        });

        document.getElementById('civCompareModal')?.addEventListener('click', (e) => {
            if (e.target.id === 'civCompareModal') {
                this.close();
            }
        });

        document.getElementById('startCivCompare')?.addEventListener('click', () => {
            this.doCompare();
        });

        this.loadCivilizations();
    }

    async loadCivilizations() {
        try {
            const data = await API.getCivilizations();
            this.civilizations = data || [];
            this.renderCivilizationList();
        } catch (err) {
            console.error('加载文明列表失败:', err);
        }
    }

    renderCivilizationList() {
        const list = document.getElementById('civSelectionList');
        if (!list) return;

        list.innerHTML = this.civilizations.map(civ => `
            <label class="civ-checkbox-item">
                <input type="checkbox" value="${civ.id}" ${this.selectedIds.includes(civ.id) ? 'checked' : ''}>
                <span class="civ-name">${civ.name_cn || civ.name}</span>
                <span class="civ-region">${civ.region || ''}</span>
            </label>
        `).join('');

        list.querySelectorAll('input[type="checkbox"]').forEach(cb => {
            cb.addEventListener('change', (e) => {
                const id = parseInt(e.target.value);
                if (e.target.checked) {
                    if (this.selectedIds.length < 5) {
                        this.selectedIds.push(id);
                    } else {
                        e.target.checked = false;
                        alert('最多选择5个文明进行对比');
                    }
                } else {
                    this.selectedIds = this.selectedIds.filter(sid => sid !== id);
                }
            });
        });
    }

    open() {
        const modal = document.getElementById('civCompareModal');
        if (modal) {
            modal.classList.add('active');
            this.isOpen = true;
            this.selectedIds = [1, 2];
            this.renderCivilizationList();
            this.resizeCanvas();
        }
    }

    close() {
        const modal = document.getElementById('civCompareModal');
        if (modal) {
            modal.classList.remove('active');
            this.isOpen = false;
        }
    }

    resizeCanvas() {
        if (this.radarCanvas) {
            const container = this.radarCanvas.parentElement;
            const size = Math.min(container.clientWidth, 350);
            this.radarCanvas.width = size;
            this.radarCanvas.height = size;
            if (this.comparisonData) {
                this.renderRadarChart();
            }
        }
    }

    async doCompare() {
        if (this.selectedIds.length < 2) {
            alert('请至少选择2个文明进行对比');
            return;
        }

        const resultDiv = document.getElementById('civCompareResult');
        if (resultDiv) {
            resultDiv.innerHTML = '<div class="loading">分析中...</div>';
        }

        try {
            const data = await API.compareCivilizations(this.selectedIds);
            this.comparisonData = data;
            this.renderComparison(data);
        } catch (err) {
            console.error('文明对比失败:', err);
            if (resultDiv) {
                resultDiv.innerHTML = `<div class="error">对比失败: ${err.message}</div>`;
            }
        }
    }

    renderComparison(data) {
        const resultDiv = document.getElementById('civCompareResult');
        if (!resultDiv) return;

        const indicators = data.indicators || [];
        const radarData = data.radar_data || [];
        const civilizations = data.civilizations || [];

        resultDiv.innerHTML = `
            <div class="civ-radar-container">
                <canvas id="civRadarChart"></canvas>
            </div>
            <div class="civ-compare-notes">
                <h4>规划思想对比分析</h4>
                ${data.analysis_notes ? `<p>${data.analysis_notes}</p>` : '<p>不同文明的城市规划反映了各自独特的文化、政治和地理特征。</p>'}
            </div>
            <div class="civ-legend">
                ${(civilizations || []).map((civ, idx) => `
                    <div class="civ-legend-item">
                        <div class="civ-legend-color" style="background: ${CONFIG.CIVILIZATION_COLORS[idx % CONFIG.CIVILIZATION_COLORS.length]}"></div>
                        <span>${civ.name_cn || civ.name}</span>
                    </div>
                `).join('')}
            </div>
        `;

        this.radarCanvas = document.getElementById('civRadarChart');
        this.radarCtx = this.radarCanvas?.getContext('2d');
        this.resizeCanvas();

        if (this.radarCtx && radarData.length > 0) {
            this.renderRadarChart(indicators, radarData);
        }
    }

    renderRadarChart(indicators, radarData) {
        if (!this.radarCtx || !indicators || !radarData) return;

        const canvas = this.radarCanvas;
        const ctx = this.radarCtx;
        const W = canvas.width;
        const H = canvas.height;
        const centerX = W / 2;
        const centerY = H / 2;
        const radius = Math.min(W, H) / 2 - 50;

        ctx.clearRect(0, 0, W, H);

        const N = indicators.length;
        if (N === 0) return;

        const levels = 5;
        ctx.strokeStyle = '#ecf0f1';
        ctx.fillStyle = '#bdc3c7';
        ctx.font = '10px sans-serif';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';

        for (let l = 1; l <= levels; l++) {
            const r = (radius * l) / levels;
            ctx.beginPath();
            for (let i = 0; i < N; i++) {
                const angle = (i * 2 * Math.PI) / N - Math.PI / 2;
                const x = centerX + r * Math.cos(angle);
                const y = centerY + r * Math.sin(angle);
                if (i === 0) ctx.moveTo(x, y);
                else ctx.lineTo(x, y);
            }
            ctx.closePath();
            ctx.stroke();
        }

        for (let i = 0; i < N; i++) {
            const angle = (i * 2 * Math.PI) / N - Math.PI / 2;
            const x = centerX + radius * Math.cos(angle);
            const y = centerY + radius * Math.sin(angle);

            ctx.beginPath();
            ctx.moveTo(centerX, centerY);
            ctx.lineTo(x, y);
            ctx.strokeStyle = '#ecf0f1';
            ctx.stroke();

            const labelR = radius + 18;
            const lx = centerX + labelR * Math.cos(angle);
            const ly = centerY + labelR * Math.sin(angle);
            ctx.fillStyle = '#2c3e50';
            ctx.fillText(indicators[i] || '', lx, ly);
        }

        radarData.forEach((dataset, dsIdx) => {
            const color = CONFIG.CIVILIZATION_COLORS[dsIdx % CONFIG.CIVILIZATION_COLORS.length];
            const values = dataset.values || [];

            ctx.strokeStyle = color;
            ctx.fillStyle = color;
            ctx.globalAlpha = 0.3;
            ctx.lineWidth = 2;

            ctx.beginPath();
            for (let i = 0; i < N; i++) {
                const value = values[i] || 0;
                const angle = (i * 2 * Math.PI) / N - Math.PI / 2;
                const r = radius * (value / 100);
                const x = centerX + r * Math.cos(angle);
                const y = centerY + r * Math.sin(angle);
                if (i === 0) ctx.moveTo(x, y);
                else ctx.lineTo(x, y);
            }
            ctx.closePath();
            ctx.fill();
            ctx.globalAlpha = 1;
            ctx.stroke();

            for (let i = 0; i < N; i++) {
                const value = values[i] || 0;
                const angle = (i * 2 * Math.PI) / N - Math.PI / 2;
                const r = radius * (value / 100);
                const x = centerX + r * Math.cos(angle);
                const y = centerY + r * Math.sin(angle);

                ctx.beginPath();
                ctx.arc(x, y, 3, 0, 2 * Math.PI);
                ctx.fillStyle = color;
                ctx.fill();
            }
        });
    }
}
