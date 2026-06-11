class App {
    constructor() {
        this.cityMap = new CityMap();
        this.timeline = new Timeline();
        this.morphologyPanel = new MorphologyPanel();
        this.trendAnalyzer = new TrendAnalyzer();
        this.compareModal = new CompareModal();
        this.populationPanel = new PopulationPanel();
        this.defensePanel = new DefensePanel();
        this.landUsePanel = new LandUsePanel();
        this.civilizationCompare = new CivilizationCompare();
        
        this.dynasties = [];
        this.allSites = [];
        this.currentSite = null;
    }

    async init() {
        this.cityMap.init();
        this.morphologyPanel.init();
        this.trendAnalyzer.init();
        this.populationPanel.init();
        this.defensePanel.init();
        this.landUsePanel.init();
        this.civilizationCompare.init();

        this.cityMap.setOnZoneClick((zone) => this.showZoneDetail(zone));
        this.cityMap.setOnBuildingClick((building) => this.showBuildingDetail(building));

        this.populationPanel.onDataLoaded = (data) => {
            this.cityMap.setPopulationData(data);
        };

        this.defensePanel.onDataLoaded = (data) => {
            this.cityMap.setDefenseData(data);
        };

        try {
            this.dynasties = await API.getDynasties();
            this.timeline.init(this.dynasties);
            this.populateDynastySelect();
            
            this.timeline.setOnDynastyChange((dynasty) => this.onDynastyChange(dynasty));
        } catch (error) {
            console.error('加载朝代数据失败:', error);
        }

        try {
            this.allSites = await API.getCitySites();
            this.populateSiteSelect(this.allSites);
            this.compareModal.init(this.allSites);
            
            if (this.allSites.length > 0) {
                this.selectSite(this.allSites[0]);
            }
        } catch (error) {
            console.error('加载城市遗址数据失败:', error);
        }

        document.getElementById('dynastySelect').addEventListener('change', (e) => {
            this.filterSitesByDynasty(e.target.value);
        });

        document.getElementById('siteSelect').addEventListener('change', (e) => {
            const siteId = parseInt(e.target.value);
            const site = this.allSites.find(s => s.id === siteId);
            if (site) {
                this.selectSite(site);
            }
        });

        window.addEventListener('resize', () => {
            this.landUsePanel.resize();
        });
    }

    populateDynastySelect() {
        const select = document.getElementById('dynastySelect');
        const options = this.dynasties
            .sort((a, b) => a.start_year - b.start_year)
            .map(d => `<option value="${d.id}">${d.name}</option>`)
            .join('');
        
        select.innerHTML = '<option value="">全部朝代</option>' + options;
    }

    populateSiteSelect(sites) {
        const select = document.getElementById('siteSelect');
        const options = sites
            .sort((a, b) => (a.dynasty_name || '').localeCompare(b.dynasty_name || ''))
            .map(s => `<option value="${s.id}">${s.dynasty_name || ''} - ${s.name}</option>`)
            .join('');
        
        select.innerHTML = '<option value="">选择城市遗址</option>' + options;
    }

    filterSitesByDynasty(dynastyId) {
        if (!dynastyId) {
            this.populateSiteSelect(this.allSites);
            return;
        }
        
        const filtered = this.allSites.filter(s => s.dynasty_id === parseInt(dynastyId));
        this.populateSiteSelect(filtered);
        
        if (filtered.length > 0) {
            this.selectSite(filtered[0]);
        } else {
            this.cityMap.loadSite(null);
            this.currentSite = null;
            this.updateSiteInfo(null);
            this.morphologyPanel.setSite(null);
            this.populationPanel.setSite(null);
            this.defensePanel.setSite(null);
            this.landUsePanel.setSite(null);
        }
    }

    onDynastyChange(dynasty) {
        document.getElementById('dynastySelect').value = dynasty.id;
        this.filterSitesByDynasty(dynasty.id);
    }

    selectSite(site) {
        this.currentSite = site;
        document.getElementById('siteSelect').value = site.id;
        
        this.cityMap.loadSite(site);
        this.updateSiteInfo(site);
        this.morphologyPanel.setSite(site.id);
        this.populationPanel.setSite(site.id);
        this.defensePanel.setSite(site.id);
        this.landUsePanel.setSite(site.id);
        
        this.clearDetailPanels();
        
        setTimeout(() => {
            this.landUsePanel.resize();
        }, 100);
    }

    updateSiteInfo(site) {
        const container = document.getElementById('siteInfo');
        
        if (!site) {
            container.innerHTML = '<p class="placeholder">请选择一个城市遗址</p>';
            return;
        }

        container.innerHTML = `
            <div class="info-item">
                <div class="info-label">名称</div>
                <div class="info-value">${site.name}</div>
            </div>
            <div class="info-item">
                <div class="info-label">朝代</div>
                <div class="info-value">${site.dynasty_name || '未知'}</div>
            </div>
            <div class="info-item">
                <div class="info-label">位置</div>
                <div class="info-value">${site.location || '未知'}</div>
            </div>
            <div class="info-item">
                <div class="info-label">中心坐标</div>
                <div class="info-value">${site.center_longitude.toFixed(4)}, ${site.center_latitude.toFixed(4)}</div>
            </div>
            <div class="info-item">
                <div class="info-label">面积</div>
                <div class="info-value">${site.area_sq_km ? site.area_sq_km.toFixed(2) + ' km²' : '未知'}</div>
            </div>
            <div class="info-item">
                <div class="info-label">估算人口</div>
                <div class="info-value">${site.estimated_population ? site.estimated_population.toLocaleString() + ' 人' : '未知'}</div>
            </div>
            ${site.description ? `
            <div class="info-item">
                <div class="info-label">描述</div>
                <div class="info-value" style="font-weight: normal; font-size: 12px;">${site.description}</div>
            </div>
            ` : ''}
            ${site.archaeological_notes ? `
            <div class="info-item">
                <div class="info-label">考古备注</div>
                <div class="info-value" style="font-weight: normal; font-size: 12px;">${site.archaeological_notes}</div>
            </div>
            ` : ''}
        `;
    }

    showZoneDetail(zone) {
        const container = document.getElementById('zoneDetail');
        
        const color = CONFIG.ZONE_COLORS[zone.zone_type] || CONFIG.ZONE_COLORS.default;
        
        container.innerHTML = `
            <div class="detail-title" style="color: ${color}">
                ${zone.name || zone.zone_type}
            </div>
            <div class="detail-section">
                <h4>类型</h4>
                <p>${zone.zone_type}</p>
            </div>
            <div class="detail-section">
                <h4>置信度</h4>
                <div class="confidence-bar">
                    <div class="confidence-fill" style="width: ${(zone.confidence_level || 0) * 100}%"></div>
                </div>
                <p style="margin-top: 4px; font-size: 11px;">${((zone.confidence_level || 0) * 100).toFixed(0)}%</p>
            </div>
            ${zone.description ? `
            <div class="detail-section">
                <h4>描述</h4>
                <p>${zone.description}</p>
            </div>
            ` : ''}
            ${zone.archaeological_findings ? `
            <div class="detail-section">
                <h4>考古发现</h4>
                <p>${zone.archaeological_findings}</p>
            </div>
            ` : ''}
            ${zone.functional_inference ? `
            <div class="detail-section">
                <h4>功能推断</h4>
                <p>${zone.functional_inference}</p>
            </div>
            ` : ''}
        `;
    }

    showBuildingDetail(building) {
        const container = document.getElementById('buildingDetail');
        
        const color = CONFIG.BUILDING_COLORS[building.building_type] || CONFIG.BUILDING_COLORS.default;
        
        container.innerHTML = `
            <div class="detail-title" style="color: ${color}">
                ${building.name || building.building_type}
            </div>
            <div class="detail-section">
                <h4>类型</h4>
                <p>${building.building_type || '未知'}</p>
            </div>
            <div class="detail-section">
                <h4>建筑面积</h4>
                <p>${building.area_sq_m ? building.area_sq_m.toFixed(1) + ' 平方米' : '未知'}</p>
            </div>
            <div class="detail-section">
                <h4>房间数</h4>
                <p>${building.rooms_count || '未知'}</p>
            </div>
            ${building.description ? `
            <div class="detail-section">
                <h4>描述</h4>
                <p>${building.description}</p>
            </div>
            ` : ''}
            ${building.archaeological_findings ? `
            <div class="detail-section">
                <h4>出土文物</h4>
                <p>${building.archaeological_findings}</p>
            </div>
            ` : ''}
        `;
    }

    clearDetailPanels() {
        document.getElementById('zoneDetail').innerHTML = 
            '<p class="placeholder">点击地图上的功能区查看详情</p>';
        document.getElementById('buildingDetail').innerHTML = 
            '<p class="placeholder">点击地图上的建筑基址查看详情</p>';
    }
}

document.addEventListener('DOMContentLoaded', () => {
    const app = new App();
    app.init();
});
