const API = {
    async get(url) {
        try {
            const response = await fetch(`${CONFIG.API_BASE_URL}${url}`);
            const data = await response.json();
            if (data.success) {
                return data.data;
            } else {
                throw new Error(data.message || '请求失败');
            }
        } catch (error) {
            console.error('API GET Error:', error);
            throw error;
        }
    },

    async post(url, body) {
        try {
            const response = await fetch(`${CONFIG.API_BASE_URL}${url}`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(body)
            });
            const data = await response.json();
            if (data.success) {
                return data.data;
            } else {
                throw new Error(data.message || '请求失败');
            }
        } catch (error) {
            console.error('API POST Error:', error);
            throw error;
        }
    },

    getDynasties() {
        return this.get('/dynasties');
    },

    getCitySites() {
        return this.get('/sites');
    },

    getCitySiteById(id) {
        return this.get(`/sites/${id}`);
    },

    getSitesByDynasty(dynastyId) {
        return this.get(`/sites/dynasty/${dynastyId}`);
    },

    getFunctionalZones(siteId) {
        return this.get(`/zones/${siteId}`);
    },

    getRoads(siteId) {
        return this.get(`/roads/${siteId}`);
    },

    getBuildings(siteId) {
        return this.get(`/buildings/${siteId}`);
    },

    getPopulation(siteId) {
        return this.get(`/population/${siteId}`);
    },

    getMorphology(siteId) {
        return this.get(`/morphology/${siteId}`);
    },

    analyzeMorphology(siteId) {
        return this.post(`/morphology/analyze/${siteId}`, {});
    },

    getRoadSyntax(siteId) {
        return this.get(`/syntax/roads/${siteId}`);
    },

    analyzeTrends(indicator, dynastyIds) {
        return this.post('/trends/analyze', {
            indicator: indicator,
            dynasty_ids: dynastyIds
        });
    },

    getTrends() {
        return this.get('/trends');
    },

    compareSites(siteIds) {
        return this.post('/compare', { site_ids: siteIds });
    },

    analyzePopulation(siteId, model) {
        let url = `/population/analyze/${siteId}`;
        if (model) {
            url += `?model=${model}`;
        }
        return this.get(url);
    },

    getPopulationDistribution(siteId) {
        return this.get(`/population/distribution/${siteId}`);
    },

    analyzeDefense(siteId) {
        return this.get(`/defense/analyze/${siteId}`);
    },

    getDefenseAnalysis(siteId) {
        return this.get(`/defense/${siteId}`);
    },

    getLandUseTimeline(siteId) {
        return this.get(`/landuse/${siteId}`);
    },

    getLandUseTrend(siteId) {
        return this.get(`/landuse/trend/${siteId}`);
    },

    getCivilizations() {
        return this.get('/civilizations');
    },

    compareCivilizations(civilizationIds) {
        return this.post('/civilizations/compare', {
            civilization_ids: civilizationIds
        });
    },

    getCivilizationSites(civId) {
        return this.get(`/civilizations/${civId}/sites`);
    }
};
