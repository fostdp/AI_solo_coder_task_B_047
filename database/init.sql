-- ========================================
-- 古代城市遗址空间结构复原与形态演化分析系统
-- 数据库初始化脚本
-- ========================================

-- 启用PostGIS扩展
CREATE EXTENSION IF NOT EXISTS postgis;
CREATE EXTENSION IF NOT EXISTS postgis_topology;

-- ========================================
-- 朝代表
-- ========================================
CREATE TABLE IF NOT EXISTS dynasties (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50) NOT NULL UNIQUE,
    start_year INTEGER NOT NULL,
    end_year INTEGER NOT NULL,
    period VARCHAR(20) NOT NULL,
    description TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- ========================================
-- 城市遗址表
-- ========================================
CREATE TABLE IF NOT EXISTS city_sites (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    dynasty_id INTEGER NOT NULL REFERENCES dynasties(id),
    location VARCHAR(200),
    center_longitude DOUBLE PRECISION NOT NULL,
    center_latitude DOUBLE PRECISION NOT NULL,
    estimated_population INTEGER,
    area_sq_km DOUBLE PRECISION,
    description TEXT,
    archaeological_notes TEXT,
    geom geometry(MultiPolygon, 4326),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_city_sites_dynasty ON city_sites(dynasty_id);
CREATE INDEX IF NOT EXISTS idx_city_sites_geom ON city_sites USING GIST(geom);

-- ========================================
-- 功能区表
-- ========================================
CREATE TABLE IF NOT EXISTS functional_zones (
    id SERIAL PRIMARY KEY,
    city_site_id INTEGER NOT NULL REFERENCES city_sites(id) ON DELETE CASCADE,
    zone_type VARCHAR(50) NOT NULL,
    name VARCHAR(100),
    description TEXT,
    archaeological_findings TEXT,
    functional_inference TEXT,
    confidence_level DOUBLE PRECISION DEFAULT 0.5,
    geom geometry(MultiPolygon, 4326),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_functional_zones_city ON functional_zones(city_site_id);
CREATE INDEX IF NOT EXISTS idx_functional_zones_type ON functional_zones(zone_type);
CREATE INDEX IF NOT EXISTS idx_functional_zones_geom ON functional_zones USING GIST(geom);

-- ========================================
-- 道路网络表
-- ========================================
CREATE TABLE IF NOT EXISTS roads (
    id SERIAL PRIMARY KEY,
    city_site_id INTEGER NOT NULL REFERENCES city_sites(id) ON DELETE CASCADE,
    road_name VARCHAR(100),
    road_type VARCHAR(50),
    width DOUBLE PRECISION,
    description TEXT,
    geom geometry(MultiLineString, 4326),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_roads_city ON roads(city_site_id);
CREATE INDEX IF NOT EXISTS idx_roads_geom ON roads USING GIST(geom);

-- ========================================
-- 道路节点表（用于空间句法分析）
-- ========================================
CREATE TABLE IF NOT EXISTS road_nodes (
    id SERIAL PRIMARY KEY,
    city_site_id INTEGER NOT NULL REFERENCES city_sites(id) ON DELETE CASCADE,
    road_ids INTEGER[],
    geom geometry(Point, 4326),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_road_nodes_city ON road_nodes(city_site_id);
CREATE INDEX IF NOT EXISTS idx_road_nodes_geom ON road_nodes USING GIST(geom);

-- ========================================
-- 建筑基址表
-- ========================================
CREATE TABLE IF NOT EXISTS building_foundations (
    id SERIAL PRIMARY KEY,
    city_site_id INTEGER NOT NULL REFERENCES city_sites(id) ON DELETE CASCADE,
    building_type VARCHAR(50),
    name VARCHAR(100),
    area_sq_m DOUBLE PRECISION,
    rooms_count INTEGER,
    description TEXT,
    archaeological_findings TEXT,
    geom geometry(Point, 4326),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_building_foundations_city ON building_foundations(city_site_id);
CREATE INDEX IF NOT EXISTS idx_building_foundations_type ON building_foundations(building_type);
CREATE INDEX IF NOT EXISTS idx_building_foundations_geom ON building_foundations USING GIST(geom);

-- ========================================
-- 历史地图表
-- ========================================
CREATE TABLE IF NOT EXISTS historical_maps (
    id SERIAL PRIMARY KEY,
    city_site_id INTEGER REFERENCES city_sites(id),
    dynasty_id INTEGER REFERENCES dynasties(id),
    map_name VARCHAR(200) NOT NULL,
    map_type VARCHAR(50),
    source VARCHAR(200),
    image_url VARCHAR(500),
    georeferenced BOOLEAN DEFAULT FALSE,
    bounds geometry(Polygon, 4326),
    description TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- ========================================
-- 人口估算数据表
-- ========================================
CREATE TABLE IF NOT EXISTS population_estimates (
    id SERIAL PRIMARY KEY,
    city_site_id INTEGER NOT NULL REFERENCES city_sites(id) ON DELETE CASCADE,
    estimate_year INTEGER,
    population_min INTEGER,
    population_max INTEGER,
    population_mean INTEGER,
    estimation_method VARCHAR(100),
    source VARCHAR(200),
    confidence_level DOUBLE PRECISION,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_population_estimates_city ON population_estimates(city_site_id);

-- ========================================
-- 城市形态分析结果表
-- ========================================
CREATE TABLE IF NOT EXISTS morphology_analyses (
    id SERIAL PRIMARY KEY,
    city_site_id INTEGER NOT NULL REFERENCES city_sites(id) ON DELETE CASCADE,
    analysis_date TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    integration_global DOUBLE PRECISION,
    integration_local DOUBLE PRECISION,
    choice_global DOUBLE PRECISION,
    choice_local DOUBLE PRECISION,
    mean_depth DOUBLE PRECISION,
    total_depth DOUBLE PRECISION,
    connectivity DOUBLE PRECISION,
    boundary_fractal_dimension DOUBLE PRECISION,
    road_network_fractal_dimension DOUBLE PRECISION,
    compactness_index DOUBLE PRECISION,
    elongation_ratio DOUBLE PRECISION,
    road_density DOUBLE PRECISION,
    intersection_density DOUBLE PRECISION,
    functional_diversity DOUBLE PRECISION,
    functional_mixing DOUBLE PRECISION,
    boundary_fd_quality DOUBLE PRECISION,
    road_fd_quality DOUBLE PRECISION,
    boundary_fd_confidence_lower DOUBLE PRECISION,
    boundary_fd_confidence_upper DOUBLE PRECISION,
    road_fd_confidence_lower DOUBLE PRECISION,
    road_fd_confidence_upper DOUBLE PRECISION,
    notes TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_morphology_analyses_city ON morphology_analyses(city_site_id);

-- ========================================
-- 道路空间句法详细结果表
-- ========================================
CREATE TABLE IF NOT EXISTS road_syntax_results (
    id SERIAL PRIMARY KEY,
    road_id INTEGER NOT NULL REFERENCES roads(id) ON DELETE CASCADE,
    city_site_id INTEGER NOT NULL REFERENCES city_sites(id) ON DELETE CASCADE,
    integration DOUBLE PRECISION,
    choice DOUBLE PRECISION,
    depth DOUBLE PRECISION,
    connectivity INTEGER,
    control DOUBLE PRECISION,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_road_syntax_results_city ON road_syntax_results(city_site_id);
CREATE INDEX IF NOT EXISTS idx_road_syntax_results_road ON road_syntax_results(road_id);

-- ========================================
-- 演化趋势分析结果表
-- ========================================
CREATE TABLE IF NOT EXISTS evolution_trends (
    id SERIAL PRIMARY KEY,
    analysis_name VARCHAR(200) NOT NULL,
    indicator_name VARCHAR(100) NOT NULL,
    -- Mann-Kendall检验结果
    mk_statistic DOUBLE PRECISION,
    mk_p_value DOUBLE PRECISION,
    mk_z_score DOUBLE PRECISION,
    sen_slope DOUBLE PRECISION,
    trend_direction VARCHAR(20),
    trend_significance BOOLEAN,
    -- 时间序列数据
    time_points JSONB,
    values JSONB,
    description TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- ========================================
-- 插入朝代数据
-- ========================================
INSERT INTO dynasties (name, start_year, end_year, period, description) VALUES
('殷商', -1600, -1046, '先秦', '商朝后期，又称殷朝'),
('西周', -1046, -771, '先秦', '周朝前期，定都镐京'),
('东周-春秋', -770, -476, '先秦', '周平王东迁洛邑后的春秋时期'),
('东周-战国', -475, -221, '先秦', '战国七雄并立时期'),
('秦', -221, -207, '秦汉', '中国第一个统一的中央集权王朝'),
('西汉', -202, 8, '秦汉', '汉朝前期，定都长安'),
('东汉', 25, 220, '秦汉', '汉朝后期，定都洛阳'),
('三国-魏', 220, 265, '三国两晋南北朝', '曹魏政权'),
('三国-蜀', 221, 263, '三国两晋南北朝', '蜀汉政权'),
('三国-吴', 229, 280, '三国两晋南北朝', '东吴政权'),
('西晋', 265, 316, '三国两晋南北朝', '晋朝前期'),
('东晋', 317, 420, '三国两晋南北朝', '晋朝后期，定都建康'),
('南北朝-北魏', 386, 534, '三国两晋南北朝', '北朝魏政权'),
('隋', 581, 618, '隋唐', '结束南北朝分裂的统一王朝'),
('唐', 618, 907, '隋唐', '唐朝，中国古代鼎盛时期'),
('五代十国', 907, 960, '五代宋辽金', '唐末藩镇割据时期'),
('北宋', 960, 1127, '五代宋辽金', '宋朝前期，定都汴京'),
('南宋', 1127, 1279, '五代宋辽金', '宋朝后期，定都临安'),
('元', 1271, 1368, '元明清', '蒙古族建立的大一统王朝'),
('明', 1368, 1644, '元明清', '汉族建立的最后一个大一统王朝'),
('清', 1644, 1912, '元明清', '满族建立的最后一个封建王朝')
ON CONFLICT (name) DO NOTHING;

-- ========================================
-- 创建更新时间触发器
-- ========================================
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

DROP TRIGGER IF EXISTS update_city_sites_updated_at ON city_sites;
CREATE TRIGGER update_city_sites_updated_at
    BEFORE UPDATE ON city_sites
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ========================================
-- 文明分类表
-- ========================================
CREATE TABLE IF NOT EXISTS civilizations (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL UNIQUE,
    name_cn VARCHAR(100) NOT NULL,
    region VARCHAR(100),
    time_period VARCHAR(200),
    description TEXT,
    planning_characteristics TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- ========================================
-- 城门表
-- ========================================
CREATE TABLE IF NOT EXISTS city_gates (
    id SERIAL PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES city_sites(id) ON DELETE CASCADE,
    name VARCHAR(100),
    gate_type VARCHAR(50),
    defense_rating NUMERIC(3,2),
    geom geometry(Point, 4326),
    description TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_city_gates_site_id ON city_gates(site_id);
CREATE INDEX IF NOT EXISTS idx_city_gates_geom ON city_gates USING GIST (geom);

-- ========================================
-- 城市遗址表扩展：文明分类与防御属性
-- ========================================
ALTER TABLE city_sites ADD COLUMN IF NOT EXISTS civilization_id INTEGER REFERENCES civilizations(id);
ALTER TABLE city_sites ADD COLUMN IF NOT EXISTS terrain_type VARCHAR(50);
ALTER TABLE city_sites ADD COLUMN IF NOT EXISTS elevation NUMERIC(8,2);
ALTER TABLE city_sites ADD COLUMN IF NOT EXISTS wall_height NUMERIC(6,2);
ALTER TABLE city_sites ADD COLUMN IF NOT EXISTS wall_width NUMERIC(6,2);
ALTER TABLE city_sites ADD COLUMN IF NOT EXISTS moat_width NUMERIC(6,2);
ALTER TABLE city_sites ADD COLUMN IF NOT EXISTS num_gates INTEGER;

-- ========================================
-- 人口分布表（网格化）
-- ========================================
CREATE TABLE IF NOT EXISTS population_distributions (
    id SERIAL PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES city_sites(id) ON DELETE CASCADE,
    analysis_id INTEGER REFERENCES morphology_analyses(id) ON DELETE SET NULL,
    grid_cell_geom geometry(Polygon, 4326),
    grid_cell_centroid geometry(Point, 4326),
    population_estimate NUMERIC(12,2),
    density_per_km2 NUMERIC(12,2),
    zone_type VARCHAR(50),
    model_type VARCHAR(50),
    confidence NUMERIC(3,2),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_population_distributions_site_id ON population_distributions(site_id);
CREATE INDEX IF NOT EXISTS idx_population_distributions_analysis_id ON population_distributions(analysis_id);
CREATE INDEX IF NOT EXISTS idx_population_distributions_geom ON population_distributions USING GIST (grid_cell_geom);

-- ========================================
-- 防御体系效能分析表
-- ========================================
CREATE TABLE IF NOT EXISTS defense_analyses (
    id SERIAL PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES city_sites(id) ON DELETE CASCADE,
    overall_defense_score NUMERIC(5,2),
    visibility_analysis JSONB,
    weak_points JSONB,
    optimal_attack_routes JSONB,
    accessibility_score NUMERIC(5,2),
    gate_defense_scores JSONB,
    wall_segments JSONB,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_defense_analyses_site_id ON defense_analyses(site_id);

-- ========================================
-- 土地利用变迁表
-- ========================================
CREATE TABLE IF NOT EXISTS land_use_changes (
    id SERIAL PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES city_sites(id) ON DELETE CASCADE,
    period_name VARCHAR(100),
    period_year INTEGER,
    land_use_type VARCHAR(50),
    area_km2 NUMERIC(12,4),
    percentage NUMERIC(5,2),
    evidence_type VARCHAR(50),
    confidence NUMERIC(3,2),
    description TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_land_use_changes_site_id ON land_use_changes(site_id);
CREATE INDEX IF NOT EXISTS idx_land_use_changes_period ON land_use_changes(period_year);
CREATE INDEX IF NOT EXISTS idx_land_use_changes_type ON land_use_changes(land_use_type);

-- ========================================
-- 预置文明分类
-- ========================================
INSERT INTO civilizations (name, name_cn, region, time_period, description, planning_characteristics) VALUES
('Ancient China', '古代中国', 'East Asia', 'c. 1600 BCE - 1912 CE', 
 '古代中国文明，城市规划受礼制思想影响深远，强调中轴对称、方正布局',
 '礼制规划、中轴对称、方格路网、里坊制度、风水思想'),
('Ancient Rome', '古罗马', 'Mediterranean', 'c. 753 BCE - 476 CE',
 '古罗马文明，城市规划体现工程技术与军事组织能力，注重公共空间',
 '正交路网、公共广场、输水道、浴场、军事营寨式布局'),
('Maya', '玛雅', 'Mesoamerica', 'c. 2000 BCE - 1500 CE',
 '玛雅文明，城市与宗教天文紧密结合，多中心散布式布局',
 '金字塔神庙、天文观测、散点布局、广场仪式空间、依山就势'),
('Ancient Egypt', '古埃及', 'North Africa', 'c. 3100 BCE - 30 BCE',
 '古埃及文明，城市沿尼罗河分布，神庙与陵墓为核心',
 '沿水分布、神庙中心、陵墓城市、规整网格'),
('Mesopotamia', '美索不达米亚', 'Middle East', 'c. 3500 BCE - 539 BCE',
 '两河流域文明，城市以神庙为中心，防御体系发达',
 '神庙卫城、城墙防御、运河系统、街巷曲折')
ON CONFLICT (name) DO NOTHING;
