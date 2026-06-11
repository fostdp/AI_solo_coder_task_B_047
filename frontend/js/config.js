const CONFIG = {
    API_BASE_URL: 'http://localhost:8080/api',
    
    ZONE_COLORS: {
        '宫殿': '#e74c3c',
        '民居': '#3498db',
        '市场': '#f39c12',
        '作坊': '#9b59b6',
        '寺庙': '#27ae60',
        '官署': '#1abc9c',
        '陵墓': '#8e44ad',
        '仓储': '#e67e22',
        'default': '#95a5a6'
    },
    
    BUILDING_COLORS: {
        '宫殿建筑': '#e74c3c',
        '民居建筑': '#3498db',
        '商铺建筑': '#f39c12',
        '作坊建筑': '#9b59b6',
        '寺庙建筑': '#27ae60',
        '官署建筑': '#1abc9c',
        '塔楼': '#8e44ad',
        '城门': '#e67e22',
        'default': '#95a5a6'
    },
    
    SYNTAX_COLOR_SCALE: [
        '#313695', '#4575b4', '#74add1', '#abd9e9', '#e0f3f8',
        '#ffffbf', '#fee090', '#fdae61', '#f46d43', '#d73027', '#a50026'
    ],
    
    DYNASTY_ORDER: [
        '殷商', '西周', '东周-春秋', '东周-战国', '秦', '西汉', '东汉',
        '三国-魏', '三国-蜀', '三国-吴', '西晋', '东晋', '南北朝-北魏',
        '隋', '唐', '五代十国', '北宋', '南宋', '元', '明', '清'
    ],
    
    MORPHOLOGY_INDICATORS: {
        'integration_global': '全局整合度',
        'integration_local': '局部整合度',
        'choice_global': '全局选择度',
        'choice_local': '局部选择度',
        'mean_depth': '平均深度',
        'connectivity': '连接度',
        'boundary_fractal_dimension': '边界分形维数',
        'road_network_fractal_dimension': '路网分形维数',
        'compactness_index': '紧凑度指数',
        'elongation_ratio': '延展率',
        'road_density': '道路密度',
        'intersection_density': '交叉口密度',
        'functional_diversity': '功能多样性',
        'functional_mixing': '功能混合度'
    },

    RENDER: {
        CANVAS_PADDING: 0.5,
        LOD_ZOOM_OVERVIEW: 11,
        LOD_ZOOM_LOW: 13,
        LOD_ZOOM_MEDIUM: 15,
        THROTTLE_MS: 150,
        BUILDING_AGGREGATION_GRID: 0.005,
        MAP_CENTER: [34.0, 108.0],
        MAP_DEFAULT_ZOOM: 12,
        MAP_MIN_ZOOM: 8,
        MAP_MAX_ZOOM: 18,
        FRACTAL_GRID_LEVELS_HIGH: [1, 2, 4, 8, 16],
        FRACTAL_GRID_LEVELS_LOW: [1, 2, 4, 8],
        POPULATION_HEATMAP_OPACITY: 0.7,
        POPULATION_GRID_SIZE: 0.002,
        DEFENSE_ATTACK_ROUTE_COLOR: '#e74c3c',
        DEFENSE_WEAK_POINT_COLOR: '#e67e22',
        DEFENSE_GATE_COLOR: '#8e44ad'
    },

    POPULATION_COLOR_SCALE: [
        '#ffffb2', '#fed976', '#feb24c', '#fd8d3c',
        '#fc4e2a', '#e31a1c', '#b10026'
    ],

    LAND_USE_COLORS: {
        'urban': '#e74c3c',
        'farmland': '#27ae60',
        'forest': '#2ecc71',
        'grassland': '#f1c40f',
        'wetland': '#3498db',
        'water': '#2980b9',
        'wasteland': '#95a5a6',
        'settlement': '#e67e22',
        'default': '#7f8c8d'
    },

    LAND_USE_LABELS: {
        'urban': '城市建成区',
        'farmland': '农田',
        'forest': '森林',
        'grassland': '草地',
        'wetland': '湿地',
        'water': '水体',
        'wasteland': '荒地',
        'settlement': '居民点'
    },

    CIVILIZATION_COLORS: [
        '#e74c3c', '#3498db', '#27ae60', '#f39c12', '#9b59b6',
        '#1abc9c', '#e67e22', '#34495e'
    ],

    VIEW_MODES: [
        { id: 'plan', name: '平面图', icon: '🗺️' },
        { id: 'syntax', name: '空间句法', icon: '🔗' },
        { id: 'fractal', name: '分形维数', icon: '🔲' },
        { id: 'population', name: '人口热力', icon: '👥' },
        { id: 'defense', name: '防御分析', icon: '🏰' }
    ]
};
