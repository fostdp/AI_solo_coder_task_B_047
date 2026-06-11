class CityMap {
    constructor() {
        this.map = null;
        this.currentSite = null;
        this.wallsLayer = null;
        this.roadsLayer = null;
        this.zonesLayer = null;
        this.buildingsLayer = null;
        this.syntaxLayer = null;
        this.populationLayer = null;
        this.defenseLayer = null;
        this.canvasRenderer = null;
        this.currentView = 'plan';
        this.zonesData = [];
        this.roadsData = [];
        this.buildingsData = [];
        this.roadSyntaxData = [];
        this.populationData = null;
        this.defenseData = null;
        this.onZoneClick = null;
        this.onBuildingClick = null;
        this.renderFrame = null;
        this.lastRenderZoom = -1;
        this.lastRenderBounds = null;
        this.loadingPromises = new Map();
    }

    init() {
        const R = CONFIG.RENDER;
        this.canvasRenderer = L.canvas({ padding: R.CANVAS_PADDING });

        this.map = L.map('map', {
            center: R.MAP_CENTER,
            zoom: R.MAP_DEFAULT_ZOOM,
            minZoom: R.MAP_MIN_ZOOM,
            maxZoom: R.MAP_MAX_ZOOM,
            preferCanvas: true,
            renderer: this.canvasRenderer
        });

        L.tileLayer('https://{s}.basemaps.cartocdn.com/light_all/{z}/{x}/{y}{r}.png', {
            attribution: '&copy; OpenStreetMap contributors &copy; CARTO',
            maxZoom: 19
        }).addTo(this.map);

        this.wallsLayer = L.layerGroup().addTo(this.map);
        this.roadsLayer = L.layerGroup().addTo(this.map);
        this.zonesLayer = L.layerGroup().addTo(this.map);
        this.buildingsLayer = L.layerGroup().addTo(this.map);
        this.syntaxLayer = L.layerGroup().addTo(this.map);
        this.populationLayer = L.layerGroup().addTo(this.map);
        this.defenseLayer = L.layerGroup().addTo(this.map);

        this.setupLayerControls();
        this.setupPerformanceEvents();
    }

    setupPerformanceEvents() {
        const R = CONFIG.RENDER;
        this.map.on('movestart', () => {
            if (this.renderFrame) {
                cancelAnimationFrame(this.renderFrame);
                this.renderFrame = null;
            }
        });

        this.map.on('moveend zoomend', L.Util.throttle(() => {
            this.scheduleRender();
        }, R.THROTTLE_MS, this));
    }

    scheduleRender() {
        if (this.renderFrame) return;
        this.renderFrame = requestAnimationFrame(() => {
            this.renderFrame = null;
            this.renderVisible();
        });
    }

    getRenderLevel(zoom) {
        const R = CONFIG.RENDER;
        if (zoom < R.LOD_ZOOM_OVERVIEW) return 'overview';
        if (zoom < R.LOD_ZOOM_LOW) return 'low';
        if (zoom < R.LOD_ZOOM_MEDIUM) return 'medium';
        return 'high';
    }

    filterFeaturesInView(bounds, features, geomKey = 'geom') {
        if (!bounds || !features) return [];
        return features.filter(f => {
            if (!f[geomKey] || !f[geomKey].coordinates) return false;
            const bb = this.featureBBox(f[geomKey]);
            if (!bb) return true;
            return bounds.overlaps(L.latLngBounds([bb.minY, bb.minX], [bb.maxY, bb.maxX]));
        });
    }

    featureBBox(geom) {
        if (!geom || !geom.coordinates) return null;
        let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
        const traverse = (coords) => {
            if (typeof coords[0] === 'number') {
                minX = Math.min(minX, coords[0]);
                maxX = Math.max(maxX, coords[0]);
                minY = Math.min(minY, coords[1]);
                maxY = Math.max(maxY, coords[1]);
            } else {
                coords.forEach(c => traverse(c));
            }
        };
        traverse(geom.coordinates);
        if (!isFinite(minX)) return null;
        return { minX, maxX, minY, maxY };
    }

    renderVisible() {
        const zoom = this.map.getZoom();
        const bounds = this.map.getBounds();
        const level = this.getRenderLevel(zoom);

        if (this.lastRenderZoom === zoom && this.lastRenderBounds &&
            this.lastRenderBounds.equals(bounds)) {
            return;
        }
        this.lastRenderZoom = zoom;
        this.lastRenderBounds = bounds;

        if (level === 'overview') {
            this.renderOverview(bounds);
        } else if (level === 'low') {
            this.renderLowDetail(bounds);
        } else if (level === 'medium') {
            this.renderMediumDetail(bounds);
        } else {
            this.renderHighDetail(bounds);
        }
    }

    renderOverview(bounds) {
        this.roadsLayer.clearLayers();
        this.zonesLayer.clearLayers();
        this.buildingsLayer.clearLayers();
        this.renderAggregateBuildings(bounds);
    }

    renderLowDetail(bounds) {
        this.roadsLayer.clearLayers();
        this.zonesLayer.clearLayers();
        this.buildingsLayer.clearLayers();

        const visibleZones = this.filterFeaturesInView(bounds, this.zonesData);
        this.drawZones(visibleZones, true);

        this.renderAggregateBuildings(bounds);
    }

    renderMediumDetail(bounds) {
        this.buildingsLayer.clearLayers();

        const visibleRoads = this.filterFeaturesInView(bounds, this.roadsData);
        this.roadsLayer.clearLayers();
        this.drawRoads(visibleRoads, true);

        const visibleZones = this.filterFeaturesInView(bounds, this.zonesData);
        this.zonesLayer.clearLayers();
        this.drawZones(visibleZones, true);

        this.renderAggregateBuildings(bounds);
    }

    renderHighDetail(bounds) {
        const visibleRoads = this.filterFeaturesInView(bounds, this.roadsData);
        this.roadsLayer.clearLayers();
        this.drawRoads(visibleRoads, false);

        const visibleZones = this.filterFeaturesInView(bounds, this.zonesData);
        this.zonesLayer.clearLayers();
        this.drawZones(visibleZones, false);

        const visibleBuildings = this.filterFeaturesInView(bounds, this.buildingsData);
        this.buildingsLayer.clearLayers();
        this.drawBuildings(visibleBuildings, false);
    }

    renderAggregateBuildings(bounds) {
        this.buildingsLayer.clearLayers();

        const visibleBuildings = this.filterFeaturesInView(bounds, this.buildingsData);
        if (visibleBuildings.length === 0) return;

        const gridSize = CONFIG.RENDER.BUILDING_AGGREGATION_GRID;
        const clusters = new Map();

        visibleBuildings.forEach(b => {
            if (!b.geom || !b.geom.coordinates) return;
            const [lng, lat] = b.geom.coordinates;
            const gx = Math.floor(lng / gridSize);
            const gy = Math.floor(lat / gridSize);
            const key = `${gx},${gy}`;
            if (!clusters.has(key)) {
                clusters.set(key, { count: 0, types: new Map(), sumLng: 0, sumLat: 0 });
            }
            const c = clusters.get(key);
            c.count++;
            c.sumLng += lng;
            c.sumLat += lat;
            const t = b.building_type || 'unknown';
            c.types.set(t, (c.types.get(t) || 0) + 1);
        });

        clusters.forEach((c) => {
            const center = [c.sumLat / c.count, c.sumLng / c.count];
            const maxType = [...c.types.entries()].sort((a, b) => b[1] - a[1])[0];
            const color = CONFIG.BUILDING_COLORS[maxType[0]] || CONFIG.BUILDING_COLORS.default;
            const radius = Math.min(15, 4 + Math.sqrt(c.count) * 1.5);

            L.circleMarker(center, {
                renderer: this.canvasRenderer,
                radius: radius,
                fillColor: color,
                color: '#fff',
                weight: 1,
                fillOpacity: 0.75
            }).addTo(this.buildingsLayer).bindPopup(`
                <h3>建筑集群</h3>
                <p><strong>建筑数量:</strong> ${c.count}</p>
                <p><strong>主要类型:</strong> ${maxType[0]}</p>
                <p>缩放至更高等级查看详细</p>
            `);
        });
    }

    setupLayerControls() {
        document.getElementById('layerWalls').addEventListener('change', (e) => {
            if (e.target.checked) this.map.addLayer(this.wallsLayer);
            else this.map.removeLayer(this.wallsLayer);
        });

        document.getElementById('layerRoads').addEventListener('change', (e) => {
            if (e.target.checked) this.map.addLayer(this.roadsLayer);
            else this.map.removeLayer(this.roadsLayer);
        });

        document.getElementById('layerZones').addEventListener('change', (e) => {
            if (e.target.checked) this.map.addLayer(this.zonesLayer);
            else this.map.removeLayer(this.zonesLayer);
        });

        document.getElementById('layerBuildings').addEventListener('change', (e) => {
            if (e.target.checked) this.map.addLayer(this.buildingsLayer);
            else this.map.removeLayer(this.buildingsLayer);
        });

        document.querySelectorAll('.view-btn').forEach(btn => {
            btn.addEventListener('click', (e) => {
                document.querySelectorAll('.view-btn').forEach(b => b.classList.remove('active'));
                e.target.classList.add('active');
                this.switchView(e.target.dataset.view);
            });
        });
    }

    switchView(view) {
        this.currentView = view;

        this.syntaxLayer.clearLayers();
        this.populationLayer.clearLayers();
        this.defenseLayer.clearLayers();

        if (view === 'plan') {
            this.wallsLayer.eachLayer(l => l.setStyle ? l.setStyle({ opacity: 1 }) : null);
            this.buildingsLayer.eachLayer(l => l.setStyle ? l.setStyle({ opacity: 1 }) : null);
            this.scheduleRender();
        } else if (view === 'syntax') {
            this.renderSyntaxView();
        } else if (view === 'fractal') {
            this.renderFractalView();
        } else if (view === 'population') {
            this.renderPopulationHeatmap();
        } else if (view === 'defense') {
            this.renderDefenseAnalysis();
        }
    }

    loadSite(site) {
        this.currentSite = site;
        this.clearAllLayers();
        this.loadingPromises.forEach(p => p.abort && p.abort());
        this.loadingPromises.clear();

        if (!site) return;

        const center = [site.center_latitude, site.center_longitude];
        this.map.setView(center, 14, { animate: false });
        this.lastRenderZoom = -1;
        this.lastRenderBounds = null;

        if (site.geom) {
            this.drawWalls(site.geom);
        }

        this.fetchAndCache('roads_' + site.id,
            () => API.getRoads(site.id),
            (roads) => { this.roadsData = roads; this.scheduleRender(); },
            (err) => console.error('加载道路失败:', err)
        );

        this.fetchAndCache('zones_' + site.id,
            () => API.getFunctionalZones(site.id),
            (zones) => {
                this.zonesData = zones;
                this.updateZoneLegend(zones);
                this.scheduleRender();
            },
            (err) => console.error('加载功能区失败:', err)
        );

        this.fetchAndCache('buildings_' + site.id,
            () => API.getBuildings(site.id),
            (buildings) => { this.buildingsData = buildings; this.scheduleRender(); },
            (err) => console.error('加载建筑失败:', err)
        );

        this.fetchAndCache('syntax_' + site.id,
            () => API.getRoadSyntax(site.id),
            (syntax) => { this.roadSyntaxData = syntax; },
            (err) => console.error('加载空间句法数据失败:', err)
        );
    }

    fetchAndCache(key, fetchFn, onSuccess, onError) {
        if (this.loadingPromises.has(key)) return;
        let cancelled = false;
        const promise = fetchFn();
        this.loadingPromises.set(key, { abort: () => { cancelled = true; } });
        promise.then(result => {
            this.loadingPromises.delete(key);
            if (!cancelled && onSuccess) onSuccess(result);
        }).catch(err => {
            this.loadingPromises.delete(key);
            if (!cancelled && onError) onError(err);
        });
    }

    clearAllLayers() {
        this.wallsLayer.clearLayers();
        this.roadsLayer.clearLayers();
        this.zonesLayer.clearLayers();
        this.buildingsLayer.clearLayers();
        this.syntaxLayer.clearLayers();
        this.populationLayer.clearLayers();
        this.defenseLayer.clearLayers();
        this.populationData = null;
        this.defenseData = null;
    }

    drawWalls(geom) {
        if (!geom || !geom.coordinates) return;

        const polygon = L.geoJSON(geom, {
            renderer: this.canvasRenderer,
            style: {
                color: '#8b4513',
                weight: 3,
                fillColor: '#deb887',
                fillOpacity: 0.2
            }
        }).addTo(this.wallsLayer);
    }

    drawRoads(roads, simplified = false) {
        roads.forEach(road => {
            if (!road.geom || !road.geom.coordinates) return;

            const coords = road.geom.coordinates.map(c => [c[1], c[0]]);
            const weight = simplified
                ? (road.width ? Math.min(4, Math.max(1, road.width / 5)) : 2)
                : (road.width ? Math.min(8, Math.max(2, road.width / 3)) : 3);
            const opacity = simplified ? 0.6 : 0.8;

            const polyline = L.polyline(coords, {
                renderer: this.canvasRenderer,
                color: '#555',
                weight: weight,
                opacity: opacity
            }).addTo(this.roadsLayer);

            if (!simplified) {
                polyline.bindPopup(`
                    <h3>${road.road_name || '未命名道路'}</h3>
                    <p><strong>类型:</strong> ${road.road_type || '未知'}</p>
                    <p><strong>宽度:</strong> ${road.width ? road.width.toFixed(1) + ' 米' : '未知'}</p>
                    ${road.description ? `<p><strong>描述:</strong> ${road.description}</p>` : ''}
                `);
            }
        });
    }

    drawZones(zones, simplified = false) {
        zones.forEach(zone => {
            if (!zone.geom || !zone.geom.coordinates) return;

            const color = CONFIG.ZONE_COLORS[zone.zone_type] || CONFIG.ZONE_COLORS.default;
            const fillOpacity = simplified ? 0.3 : 0.5;
            const weight = simplified ? 1 : 2;

            const polygon = L.geoJSON(zone.geom, {
                renderer: this.canvasRenderer,
                style: {
                    color: color,
                    weight: weight,
                    fillColor: color,
                    fillOpacity: fillOpacity
                }
            }).addTo(this.zonesLayer);

            polygon.on('click', () => {
                if (this.onZoneClick) this.onZoneClick(zone);
            });

            if (!simplified) {
                polygon.bindTooltip(zone.name || zone.zone_type, {
                    permanent: false,
                    direction: 'center'
                });
            }
        });
    }

    drawBuildings(buildings, simplified = false) {
        buildings.forEach(building => {
            if (!building.geom || !building.geom.coordinates) return;

            const coords = [building.geom.coordinates[1], building.geom.coordinates[0]];
            const color = CONFIG.BUILDING_COLORS[building.building_type] || CONFIG.BUILDING_COLORS.default;
            const radius = simplified
                ? Math.min(6, Math.max(2, 3))
                : (building.area_sq_m ? Math.min(12, Math.max(3, Math.sqrt(building.area_sq_m) / 5)) : 5);
            const weight = simplified ? 0.5 : 1.5;

            const marker = L.circleMarker(coords, {
                renderer: this.canvasRenderer,
                radius: radius,
                fillColor: color,
                color: '#fff',
                weight: weight,
                fillOpacity: 1
            }).addTo(this.buildingsLayer);

            marker.on('click', () => {
                if (this.onBuildingClick) this.onBuildingClick(building);
            });

            if (!simplified) {
                marker.bindPopup(`
                    <h3>${building.name || building.building_type}</h3>
                    <p><strong>类型:</strong> ${building.building_type || '未知'}</p>
                    <p><strong>面积:</strong> ${building.area_sq_m ? building.area_sq_m.toFixed(1) + ' 平方米' : '未知'}</p>
                    <p><strong>房间数:</strong> ${building.rooms_count || '未知'}</p>
                `);
            }
        });
    }

    renderSyntaxView() {
        this.syntaxLayer.clearLayers();

        if (this.roadsData.length === 0 || this.roadSyntaxData.length === 0) return;

        const bounds = this.map.getBounds();
        const zoom = this.map.getZoom();
        const visibleRoads = this.filterFeaturesInView(bounds, this.roadsData);

        const integrations = this.roadSyntaxData.map(r => r.integration).filter(v => v !== null && v !== undefined);
        const minInt = Math.min(...integrations);
        const maxInt = Math.max(...integrations);
        const range = maxInt - minInt || 1;

        const visibleIds = new Set(visibleRoads.map(r => r.id));
        const visibleSyntax = this.roadSyntaxData.filter(s => visibleIds.has(s.road_id));

        visibleSyntax.forEach(syntax => {
            const road = this.roadsData.find(r => r.id === syntax.road_id);
            if (!road || !road.geom || !road.geom.coordinates) return;

            const normalizedValue = (syntax.integration - minInt) / range;
            const colorIndex = Math.floor(normalizedValue * (CONFIG.SYNTAX_COLOR_SCALE.length - 1));
            const color = CONFIG.SYNTAX_COLOR_SCALE[Math.max(0, Math.min(CONFIG.SYNTAX_COLOR_SCALE.length - 1, colorIndex))];

            const coords = road.geom.coordinates.map(c => [c[1], c[0]]);
            const polyline = L.polyline(coords, {
                renderer: this.canvasRenderer,
                color: color,
                weight: zoom >= 14 ? 5 : 3,
                opacity: 0.9
            }).addTo(this.syntaxLayer);

            polyline.bindPopup(`
                <h3>${road.road_name || '道路'}</h3>
                <p><strong>整合度:</strong> ${syntax.integration ? syntax.integration.toFixed(4) : 'N/A'}</p>
                <p><strong>选择度:</strong> ${syntax.choice ? syntax.choice.toFixed(2) : 'N/A'}</p>
                <p><strong>深度:</strong> ${syntax.depth ? syntax.depth.toFixed(2) : 'N/A'}</p>
                <p><strong>连接度:</strong> ${syntax.connectivity || 'N/A'}</p>
            `);
        });

        this.zonesLayer.eachLayer(l => { if (l.setStyle) l.setStyle({ fillOpacity: 0.2 }); });
        this.buildingsLayer.eachLayer(l => { if (l.setStyle) l.setStyle({ fillOpacity: 0.5 }); });
    }

    renderFractalView() {
        this.syntaxLayer.clearLayers();
        if (!this.currentSite || !this.currentSite.geom) return;
        this.drawFractalGrid();
    }

    drawFractalGrid() {
        if (!this.currentSite || !this.currentSite.geom) return;

        const geom = this.currentSite.geom;
        const bounds = this.getGeoJSONBounds(geom);
        if (!bounds) return;

        const R = CONFIG.RENDER;
        const zoom = this.map.getZoom();
        const levels = zoom >= 14 ? R.FRACTAL_GRID_LEVELS_HIGH : R.FRACTAL_GRID_LEVELS_LOW;
        const colors = ['#ff0000', '#ff6600', '#ffcc00', '#66ff00', '#00ccff'];

        levels.forEach((level, idx) => {
            if (idx >= colors.length) return;
            const cellSize = (bounds.maxX - bounds.minX) / level;
            for (let i = 0; i < level; i++) {
                for (let j = 0; j < level; j++) {
                    const x1 = bounds.minX + i * cellSize;
                    const y1 = bounds.minY + j * cellSize;
                    const x2 = x1 + cellSize;
                    const y2 = y1 + cellSize;

                    const rect = L.rectangle([[y1, x1], [y2, x2]], {
                        renderer: this.canvasRenderer,
                        color: colors[idx],
                        weight: 1,
                        fill: false,
                        opacity: 0.5 - idx * 0.08
                    }).addTo(this.syntaxLayer);
                }
            }
        });
    }

    getGeoJSONBounds(geom) {
        if (!geom || !geom.coordinates) return null;

        let minX = Infinity, maxX = -Infinity;
        let minY = Infinity, maxY = -Infinity;

        const traverse = (coords) => {
            if (typeof coords[0] === 'number') {
                minX = Math.min(minX, coords[0]);
                maxX = Math.max(maxX, coords[0]);
                minY = Math.min(minY, coords[1]);
                maxY = Math.max(maxY, coords[1]);
            } else {
                coords.forEach(c => traverse(c));
            }
        };

        traverse(geom.coordinates);
        if (!isFinite(minX)) return null;
        return { minX, maxX, minY, maxY };
    }

    updateZoneLegend(zones) {
        const legend = document.getElementById('zoneLegend');
        const types = new Set(zones.map(z => z.zone_type));

        legend.innerHTML = '';
        types.forEach(type => {
            const color = CONFIG.ZONE_COLORS[type] || CONFIG.ZONE_COLORS.default;
            const item = document.createElement('div');
            item.className = 'legend-item';
            item.innerHTML = `
                <div class="legend-color" style="background: ${color}"></div>
                <span>${type}</span>
            `;
            legend.appendChild(item);
        });
    }

    setOnZoneClick(callback) {
        this.onZoneClick = callback;
    }

    setOnBuildingClick(callback) {
        this.onBuildingClick = callback;
    }

    setPopulationData(data) {
        this.populationData = data;
        if (this.currentView === 'population') {
            this.populationLayer.clearLayers();
            this.renderPopulationHeatmap();
        }
    }

    setDefenseData(data) {
        this.defenseData = data;
        if (this.currentView === 'defense') {
            this.defenseLayer.clearLayers();
            this.renderDefenseAnalysis();
        }
    }

    getColorFromScale(value, maxValue, colors) {
        if (value <= 0 || maxValue <= 0) return colors[0];
        const ratio = Math.min(value / maxValue, 1);
        const idx = Math.floor(ratio * (colors.length - 1));
        return colors[Math.min(idx, colors.length - 1)];
    }

    renderPopulationHeatmap() {
        if (!this.populationData || !this.populationData.grid_cells) return;
        if (!this.currentSite) return;

        const gridCells = this.populationData.grid_cells;
        const colors = CONFIG.POPULATION_COLOR_SCALE;
        const opacity = CONFIG.RENDER.POPULATION_HEATMAP_OPACITY;
        const gridSize = CONFIG.RENDER.POPULATION_GRID_SIZE || 0.002;
        const halfGrid = gridSize / 2;

        let maxDensity = 0;
        gridCells.forEach(cell => {
            if (cell.density > maxDensity) {
                maxDensity = cell.density;
            }
        });

        gridCells.forEach(cell => {
            const color = this.getColorFromScale(cell.density, maxDensity, colors);
            const lon = cell.lon;
            const lat = cell.lat;

            const latlngs = [
                [lat + halfGrid, lon - halfGrid],
                [lat + halfGrid, lon + halfGrid],
                [lat - halfGrid, lon + halfGrid],
                [lat - halfGrid, lon - halfGrid]
            ];

            const rect = L.rectangle(latlngs, {
                renderer: this.canvasRenderer,
                fillColor: color,
                fillOpacity: opacity,
                color: color,
                weight: 0,
                stroke: false
            }).addTo(this.populationLayer);

            rect.bindTooltip(`人口密度: ${cell.density.toFixed(0)} 人/km²<br>估算人口: ${cell.population.toFixed(0)} 人<br>${cell.zone_type ? '功能区: ' + cell.zone_type : ''}`);
        });
    }

    renderDefenseAnalysis() {
        if (!this.defenseData) return;
        if (!this.currentSite) return;

        this.renderAttackRoutes();
        this.renderWeakPoints();
        this.renderGatesDefense();
    }

    renderAttackRoutes() {
        if (!this.defenseData.optimal_attack_routes) return;

        const routes = this.defenseData.optimal_attack_routes;
        const colors = ['#e74c3c', '#e67e22', '#f39c12', '#9b59b6', '#3498db', '#27ae60'];

        routes.forEach((route, idx) => {
            if (!route.route_geom || !route.route_geom.coordinates) return;

            const color = colors[idx % colors.length];
            const latlngs = route.route_geom.coordinates.map(c => [c[1], c[0]]);
            const isOptimal = idx === 0;

            const line = L.polyline(latlngs, {
                renderer: this.canvasRenderer,
                color: color,
                weight: 3,
                opacity: 0.8,
                dashArray: isOptimal ? null : '5, 5'
            }).addTo(this.defenseLayer);

            line.bindTooltip(`攻击路径 ${idx + 1}<br>危险程度: ${(route.attack_score * 100).toFixed(0)}%<br>类型: ${route.route_type || ''}`);
        });
    }

    renderWeakPoints() {
        if (!this.defenseData.weak_points) return;

        this.defenseData.weak_points.forEach((wp, idx) => {
            const marker = L.circleMarker([wp.lat, wp.lon], {
                radius: 6 + Math.min(4, wp.weakness_score * 5),
                fillColor: '#e74c3c',
                color: '#c0392b',
                weight: 2,
                fillOpacity: 0.8
            }).addTo(this.defenseLayer);

            marker.bindTooltip(`薄弱点 ${idx + 1}<br>类型: ${wp.weakness_type}<br>脆弱度: ${(wp.weakness_score * 100).toFixed(0)}%<br>${wp.description || ''}`);
        });
    }

    renderGatesDefense() {
        if (!this.defenseData.gate_defense_scores) return;
        if (!this.defenseData.gates && !this.defenseData.gate_defense_scores) return;

        const gates = this.defenseData.gates || [];
        const gateScores = this.defenseData.gate_defense_scores || [];

        if (gates.length > 0) {
            gates.forEach(gate => {
                if (!gate.geom || !gate.geom.coordinates) return;

                const scoreObj = gateScores.find(s => s.gate_id === gate.id || s.gate_name === gate.name);
                const defenseScore = scoreObj?.defense_score ?? gate.defense_rating ?? 0.5;

                let color = '#27ae60';
                if (defenseScore < 0.4) color = '#e74c3c';
                else if (defenseScore < 0.7) color = '#f39c12';

                const marker = L.circleMarker([gate.geom.coordinates[1], gate.geom.coordinates[0]], {
                    radius: 6,
                    fillColor: color,
                    color: '#2c3e50',
                    weight: 1.5,
                    fillOpacity: 0.9
                }).addTo(this.defenseLayer);

                marker.bindTooltip(`城门：${gate.name}<br>类型: ${gate.gate_type}<br>防御评分: ${(defenseScore * 100).toFixed(0)}`);
            });
        } else {
            gateScores.forEach((gs, idx) => {
                const defenseScore = gs.defense_score;
                let color = '#27ae60';
                if (defenseScore < 0.4) color = '#e74c3c';
                else if (defenseScore < 0.7) color = '#f39c12';

                const angle = (idx * 2 * Math.PI / gateScores.length) - Math.PI / 2;
                const site = this.currentSite;
                const radius = site?.area_sq_km ? Math.sqrt(site.area_sq_km / Math.PI) : 1;
                const degPerKm = 1 / 111;
                const glon = (site?.center_longitude || 116) + radius * degPerKm * Math.cos(angle);
                const glat = (site?.center_latitude || 34) + radius * degPerKm * Math.sin(angle);

                const marker = L.circleMarker([glat, glon], {
                    radius: 6,
                    fillColor: color,
                    color: '#2c3e50',
                    weight: 1.5,
                    fillOpacity: 0.9
                }).addTo(this.defenseLayer);

                marker.bindTooltip(`城门：${gs.gate_name || `第${idx + 1}门`}<br>防御评分: ${(defenseScore * 100).toFixed(0)}`);
            });
        }
    }
}
