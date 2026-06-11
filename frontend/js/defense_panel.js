class DefensePanel {
    constructor() {
        this.currentSiteId = null;
        this.defenseData = null;
        this.onDataLoaded = null;
    }

    init() {
        document.getElementById('refreshDefense')?.addEventListener('click', () => {
            this.analyze();
        });
    }

    setSite(siteId) {
        this.currentSiteId = siteId;
        this.defenseData = null;
        this.clearPanel();
        if (siteId) {
            this.analyze();
        }
    }

    clearPanel() {
        const panel = document.getElementById('defenseInfo');
        if (panel) {
            panel.innerHTML = '<div class="loading">加载中...</div>';
        }
    }

    async analyze() {
        if (!this.currentSiteId) return;

        try {
            this.clearPanel();
            const data = await API.analyzeDefense(this.currentSiteId);
            this.defenseData = data;
            this.render(data);

            if (this.onDataLoaded) {
                this.onDataLoaded(data);
            }
        } catch (err) {
            console.error('防御分析失败:', err);
            const panel = document.getElementById('defenseInfo');
            if (panel) {
                panel.innerHTML = `<div class="error">分析失败: ${err.message}</div>`;
            }
        }
    }

    render(data) {
        const panel = document.getElementById('defenseInfo');
        if (!panel) return;

        const score = data.overall_defense_score || 0;
        let scoreLevel = '优秀';
        let scoreColor = '#27ae60';
        if (score < 60) {
            scoreLevel = '较弱';
            scoreColor = '#e74c3c';
        } else if (score < 80) {
            scoreLevel = '中等';
            scoreColor = '#f39c12';
        }

        const weakPointsHtml = (data.weak_points || []).slice(0, 5).map((wp, idx) => `
            <div class="defense-item">
                <span class="defense-item-rank">${idx + 1}</span>
                <div class="defense-item-info">
                    <div class="defense-item-name">${wp.description || wp.weakness_type || '薄弱点'}</div>
                    <div class="defense-item-type">${wp.weakness_type || '-'}</div>
                </div>
                <span class="defense-item-score" style="color: ${wp.weakness_score > 0.7 ? '#e74c3c' : wp.weakness_score > 0.5 ? '#f39c12' : '#27ae60'}">
                    ${(wp.weakness_score * 100).toFixed(0)}%
                </span>
            </div>
        `).join('') || '<div class="empty">暂无数据</div>';

        const gatesHtml = (data.gate_defense_scores || []).slice(0, 6).map(g => `
            <div class="defense-gate-item">
                <span class="defense-gate-name">${g.gate_name || '城门'}</span>
                <div class="defense-gate-bar">
                    <div class="defense-gate-bar-fill" style="width: ${g.defense_score * 100}%; background: ${g.defense_score > 0.7 ? '#27ae60' : g.defense_score > 0.5 ? '#f39c12' : '#e74c3c'}"></div>
                </div>
                <span class="defense-gate-score">${(g.defense_score * 100).toFixed(0)}</span>
            </div>
        `).join('') || '';

        const factors = [];
        if (data.wall_height_score !== undefined) factors.push({ name: '城墙高度', score: data.wall_height_score * 100, weight: 25 });
        if (data.wall_width_score !== undefined) factors.push({ name: '城墙厚度', score: data.wall_width_score * 100, weight: 20 });
        if (data.moat_score !== undefined) factors.push({ name: '护城河', score: data.moat_score * 100, weight: 15 });
        if (data.gate_count_score !== undefined) factors.push({ name: '城门布局', score: data.gate_count_score * 100, weight: 20 });
        if (data.accessibility_score !== undefined) factors.push({ name: '地形可达', score: (1 - data.accessibility_score) * 100, weight: 20 });
        if (factors.length === 0) {
            factors.push({ name: '综合评估', score: score, weight: 100 });
        }

        const factorsHtml = factors.map(f => `
            <div class="defense-factor-item">
                <span class="defense-factor-name">${f.name}</span>
                <span class="defense-factor-score">${f.score.toFixed(1)}</span>
                <span class="defense-factor-weight">(${f.weight}%权重)</span>
            </div>
        `).join('');

        panel.innerHTML = `
            <div class="defense-score-section">
                <div class="defense-score-circle" style="border-color: ${scoreColor}; color: ${scoreColor}">
                    <span class="defense-score-value">${score.toFixed(1)}</span>
                    <span class="defense-score-label">分</span>
                </div>
                <div class="defense-score-level" style="color: ${scoreColor}">${scoreLevel}</div>
                <div class="defense-score-desc">综合防御效能</div>
            </div>

            <div class="defense-factors">
                <h4>防御因子分解</h4>
                ${factorsHtml}
            </div>

            <div class="defense-gates">
                <h4>城门防御评分</h4>
                ${gatesHtml}
            </div>

            <div class="defense-weak-points">
                <h4>主要薄弱点</h4>
                ${weakPointsHtml}
            </div>

            <div class="defense-attack-routes">
                <h4>攻击路径</h4>
                <div class="defense-route-info">共 ${data.optimal_attack_routes?.length || 0} 条最优攻击路径</div>
            </div>
        `;
    }
}
