#!/usr/bin/env python3
"""
古代城市遗址数据模拟脚本
生成从殷商到明清共50个城市遗址的考古发掘数据
"""

import psycopg2
from psycopg2.extras import execute_values
import random
import math
import json
import os
from datetime import datetime

random.seed(42)

DYNASTIES = [
    ("殷商", -1600, -1046, 2),
    ("西周", -1046, -771, 3),
    ("东周-春秋", -770, -476, 3),
    ("东周-战国", -475, -221, 3),
    ("秦", -221, -207, 2),
    ("西汉", -202, 8, 4),
    ("东汉", 25, 220, 3),
    ("三国-魏", 220, 265, 2),
    ("三国-蜀", 221, 263, 2),
    ("三国-吴", 229, 280, 2),
    ("西晋", 265, 316, 2),
    ("东晋", 317, 420, 2),
    ("南北朝-北魏", 386, 534, 2),
    ("隋", 581, 618, 2),
    ("唐", 618, 907, 4),
    ("五代十国", 907, 960, 3),
    ("北宋", 960, 1127, 3),
    ("南宋", 1127, 1279, 2),
    ("元", 1271, 1368, 2),
    ("明", 1368, 1644, 3),
    ("清", 1644, 1912, 3),
]

CITY_NAMES = [
    "安阳殷墟", "周原遗址", "丰镐遗址", "洛邑王城", "临淄齐故城",
    "邯郸赵故城", "咸阳城", "汉长安城", "汉魏洛阳城", "邺城",
    "建康城", "平城", "隋唐长安城", "隋唐洛阳城", "扬州城",
    "成都城", "广州城", "汴京城", "临安城", "元大都",
    "明南京城", "明中都", "明清北京城", "苏州城", "西安府城",
    "荆州城", "大同城", "太原城", "开封城", "杭州城",
    "福州城", "泉州城", "长沙城", "南昌城", "合肥城",
    "济南城", "郑州商城", "曲阜鲁国故城", "易县燕下都", "中山国灵寿城",
    "楚纪南故城", "郑韩故城", "赵邯郸故城", "齐临淄故城", "秦雍城",
    "秦栎阳城", "西汉南越王宫", "曹魏邺城", "东魏北齐邺南城", "唐大明宫遗址",
]

ZONE_TYPES = ["宫殿", "民居", "市场", "作坊", "寺庙", "官署", "陵墓", "仓储"]

BUILDING_TYPES = ["宫殿建筑", "民居建筑", "商铺建筑", "作坊建筑", "寺庙建筑", "官署建筑", "塔楼", "城门"]

def generate_polygon(center_lon, center_lat, num_sides, radius_km, irregularity=0.3):
    points = []
    for i in range(num_sides):
        angle = 2 * math.pi * i / num_sides
        r = radius_km * (1 + random.uniform(-irregularity, irregularity))
        dx = r * math.cos(angle) / 111.0
        dy = r * math.sin(angle) / (111.0 * math.cos(math.radians(center_lat)))
        points.append((center_lon + dx, center_lat + dy))
    return points

def generate_grid_roads(center_lon, center_lat, radius_km, num_streets, dynasty_idx):
    roads = []
    
    regularity = min(0.9, 0.3 + dynasty_idx * 0.03)
    
    for i in range(num_streets):
        offset = (i - num_streets // 2) * radius_km / num_streets
        y_offset = offset / (111.0 * math.cos(math.radians(center_lat)))
        
        start_lon = center_lon - radius_km / 111.0
        end_lon = center_lon + radius_km / 111.0
        y_jitter = random.uniform(-0.1, 0.1) * radius_km / (111.0 * math.cos(math.radians(center_lat)))
        
        if random.random() < regularity:
            roads.append({
                "type": "east_west",
                "coords": [
                    (start_lon, center_lat + y_offset + y_jitter),
                    (end_lon, center_lat + y_offset + y_jitter)
                ]
            })
        else:
            mid_lon = center_lon + random.uniform(-0.2, 0.2) * radius_km / 111.0
            roads.append({
                "type": "east_west",
                "coords": [
                    (start_lon, center_lat + y_offset),
                    (mid_lon, center_lat + y_offset + y_jitter),
                    (end_lon, center_lat + y_offset + y_jitter * 0.5)
                ]
            })
    
    for i in range(num_streets):
        offset = (i - num_streets // 2) * radius_km / num_streets
        x_offset = offset / 111.0
        
        start_lat = center_lat - radius_km / (111.0 * math.cos(math.radians(center_lat)))
        end_lat = center_lat + radius_km / (111.0 * math.cos(math.radians(center_lat)))
        x_jitter = random.uniform(-0.1, 0.1) * radius_km / 111.0
        
        if random.random() < regularity:
            roads.append({
                "type": "north_south",
                "coords": [
                    (center_lon + x_offset + x_jitter, start_lat),
                    (center_lon + x_offset + x_jitter, end_lat)
                ]
            })
        else:
            mid_lat = center_lat + random.uniform(-0.2, 0.2) * radius_km / (111.0 * math.cos(math.radians(center_lat)))
            roads.append({
                "type": "north_south",
                "coords": [
                    (center_lon + x_offset, start_lat),
                    (center_lon + x_offset + x_jitter, mid_lat),
                    (center_lon + x_offset + x_jitter * 0.5, end_lat)
                ]
            })
    
    return roads

def generate_functional_zones(center_lon, center_lat, radius_km, dynasty_idx, num_zones=6):
    zones = []
    zone_types = random.sample(ZONE_TYPES, min(num_zones, len(ZONE_TYPES)))
    
    for i, zone_type in enumerate(zone_types):
        angle = 2 * math.pi * i / len(zone_types) + random.uniform(-0.3, 0.3)
        dist = random.uniform(0.1, 0.6) * radius_km
        
        zone_center_lon = center_lon + dist * math.cos(angle) / 111.0
        zone_center_lat = center_lat + dist * math.sin(angle) / (111.0 * math.cos(math.radians(center_lat)))
        
        zone_radius = random.uniform(0.15, 0.35) * radius_km
        num_sides = random.randint(4, 8)
        
        polygon = generate_polygon(zone_center_lon, zone_center_lat, num_sides, zone_radius, 0.2)
        
        findings_descriptions = {
            "宫殿": "发现大型夯土台基、柱础石、瓦当等建筑构件，推测为宫殿建筑群",
            "民居": "发现小型房址、灰坑、日用陶器，推测为居民区",
            "市场": "发现较多货币、度量衡器、商铺基址，推测为市场区域",
            "作坊": "发现窑炉、冶炼遗迹、工具等手工业遗存",
            "寺庙": "发现佛像、经幢、塔基等宗教建筑遗迹",
            "官署": "发现大型建筑基址、官印、文书残片，推测为官署区",
            "陵墓": "发现墓葬群、陪葬坑、随葬品丰富",
            "仓储": "发现大型窖穴、粮仓遗迹、粮食碳化标本",
        }
        
        zones.append({
            "type": zone_type,
            "name": f"{zone_type}区",
            "polygon": polygon,
            "findings": findings_descriptions.get(zone_type, "发现古代遗迹"),
            "inference": f"根据出土文物和建筑形制，推断为{zone_type}功能区",
            "confidence": random.uniform(0.6, 0.95)
        })
    
    return zones

def generate_buildings(center_lon, center_lat, radius_km, num_buildings, dynasty_idx):
    buildings = []
    
    for i in range(num_buildings):
        angle = random.uniform(0, 2 * math.pi)
        dist = random.uniform(0, 0.8) * radius_km
        
        lon = center_lon + dist * math.cos(angle) / 111.0
        lat = center_lat + dist * math.sin(angle) / (111.0 * math.cos(math.radians(center_lat)))
        
        building_type = random.choice(BUILDING_TYPES)
        area = random.uniform(20, 500)
        rooms = random.randint(1, 15)
        
        buildings.append({
            "type": building_type,
            "name": f"{building_type}_{i+1}",
            "lon": lon,
            "lat": lat,
            "area": area,
            "rooms": rooms,
            "findings": f"出土{random.randint(5, 50)}件文物，包括陶器、瓷器、铁器等"
        })
    
    return buildings

def generate_city_site(dynasty_id, dynasty_name, city_index, dynasty_idx):
    base_lon = 108.0 + random.uniform(-10, 10)
    base_lat = 34.0 + random.uniform(-6, 6)
    
    size_factor = 0.5 + dynasty_idx * 0.12
    radius_km = random.uniform(0.8, 2.5) * size_factor
    
    area_sq_km = math.pi * radius_km * radius_km * random.uniform(0.7, 1.3)
    
    num_sides = random.randint(6, 12)
    irregularity = max(0.1, 0.5 - dynasty_idx * 0.02)
    wall_polygon = generate_polygon(base_lon, base_lat, num_sides, radius_km, irregularity)
    
    num_roads = int(6 + dynasty_idx * 0.8)
    roads = generate_grid_roads(base_lon, base_lat, radius_km * 0.8, num_roads, dynasty_idx)
    
    num_zones = min(8, int(3 + dynasty_idx * 0.3))
    zones = generate_functional_zones(base_lon, base_lat, radius_km * 0.7, dynasty_idx, num_zones)
    
    num_buildings = int(15 + dynasty_idx * 3)
    buildings = generate_buildings(base_lon, base_lat, radius_km * 0.9, num_buildings, dynasty_idx)
    
    pop_base = 5000 + dynasty_idx * 8000
    population = int(pop_base * random.uniform(0.6, 1.4))
    
    terrain_types = ["plain", "hill", "river", "wetland", "mountain"]
    terrain_weights = [0.5, 0.2, 0.15, 0.1, 0.05]
    terrain_type = random.choices(terrain_types, weights=terrain_weights)[0]
    
    elevation_base = 50
    if terrain_type == "mountain":
        elevation_base = 500
    elif terrain_type == "hill":
        elevation_base = 200
    elevation = elevation_base + random.uniform(-20, 50)
    
    wall_height = 4.0 + dynasty_idx * 0.3 + random.uniform(0, 3)
    wall_width = 3.0 + dynasty_idx * 0.25 + random.uniform(0, 2)
    
    has_moat = random.random() < 0.6
    moat_width = random.uniform(3, 15) if has_moat else 0.0
    
    num_gates = 4 if dynasty_idx < 10 else random.choice([4, 6, 8, 12])
    
    city_name = CITY_NAMES[(city_index + dynasty_idx * 2) % len(CITY_NAMES)]
    if city_index > len(CITY_NAMES):
        city_name = f"{dynasty_name}古城{city_index - len(CITY_NAMES)}号"
    
    return {
        "name": city_name,
        "dynasty_id": dynasty_id,
        "dynasty_name": dynasty_name,
        "center_lon": base_lon,
        "center_lat": base_lat,
        "area_sq_km": area_sq_km,
        "population": population,
        "wall_polygon": wall_polygon,
        "roads": roads,
        "zones": zones,
        "buildings": buildings,
        "location": f"约今{random.choice(['陕西', '河南', '山东', '山西', '河北', '湖北', '江苏', '浙江', '四川', '安徽'])}省境内",
        "description": f"{dynasty_name}时期重要城址，面积约{area_sq_km:.2f}平方公里，人口约{population}人",
        "notes": f"考古发掘面积约{random.uniform(0.1, 2.0):.2f}平方公里，出土文物{random.randint(100, 5000)}件",
        "terrain_type": terrain_type,
        "elevation": elevation,
        "wall_height": wall_height,
        "wall_width": wall_width,
        "moat_width": moat_width,
        "num_gates": num_gates,
        "radius_km": radius_km,
    }

def polygon_to_wkt(points):
    coords_str = ", ".join([f"{lon} {lat}" for lon, lat in points + [points[0]]])
    return f"SRID=4326;POLYGON(({coords_str}))"

def linestring_to_wkt(points):
    coords_str = ", ".join([f"{lon} {lat}" for lon, lat in points])
    return f"SRID=4326;LINESTRING({coords_str})"

def point_to_wkt(lon, lat):
    return f"SRID=4326;POINT({lon} {lat})"

def main():
    db_url = os.environ.get("DATABASE_URL", "dbname=ancient_city user=postgres password=postgres host=localhost port=5432")
    dynasty_filter = os.environ.get("DYNASTY_FILTER", "")
    seed = int(os.environ.get("SEED", "42"))
    random.seed(seed)

    print("="*60)
    print("古代城市遗址数据模拟器")
    print(f"数据库: {db_url.split('@')[-1] if '@' in db_url else db_url}")
    print(f"随机种子: {seed}")
    if dynasty_filter:
        print(f"朝代过滤: {dynasty_filter}")
    print("="*60)

    print("连接数据库...")
    conn = psycopg2.connect(db_url)
    cur = conn.cursor()
    
    print("获取朝代列表...")
    cur.execute("SELECT id, name, start_year FROM dynasties ORDER BY start_year")
    dynasties_db = cur.fetchall()
    dynasty_map = {name: (d_id, idx) for idx, (d_id, name, year) in enumerate(dynasties_db)}
    
    print("生成城市遗址数据...")
    total_sites = 0
    all_sites = []
    
    for dynasty_name, start_year, end_year, count in DYNASTIES:
        if dynasty_filter and dynasty_filter not in dynasty_name and dynasty_name not in dynasty_filter:
            continue
        if dynasty_name not in dynasty_map:
            print(f"警告：找不到朝代 {dynasty_name}，跳过")
            continue
        
        dynasty_id, dynasty_idx = dynasty_map[dynasty_name]
        
        for i in range(count):
            site = generate_city_site(dynasty_id, dynasty_name, total_sites + i, dynasty_idx)
            all_sites.append(site)
        
        total_sites += count
    
    print(f"共生成 {len(all_sites)} 个城市遗址")
    
    print("插入城市遗址数据...")
    site_ids = []
    for site in all_sites:
        wall_wkt = polygon_to_wkt(site["wall_polygon"])
        
        cur.execute("""
            INSERT INTO city_sites 
            (name, dynasty_id, location, center_longitude, center_latitude, 
             estimated_population, area_sq_km, description, archaeological_notes, geom,
             civilization_id, terrain_type, elevation, wall_height, wall_width, moat_width, num_gates)
            VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s)
            RETURNING id
        """, (
            site["name"],
            site["dynasty_id"],
            site["location"],
            site["center_lon"],
            site["center_lat"],
            site["population"],
            site["area_sq_km"],
            site["description"],
            site["notes"],
            wall_wkt,
            1,
            site["terrain_type"],
            site["elevation"],
            site["wall_height"],
            site["wall_width"],
            site["moat_width"],
            site["num_gates"],
        ))
        
        site_id = cur.fetchone()[0]
        site_ids.append(site_id)
        site["db_id"] = site_id
    
    conn.commit()
    print(f"已插入 {len(site_ids)} 个城市遗址")
    
    print("插入功能区数据...")
    zone_count = 0
    for site in all_sites:
        site_id = site["db_id"]
        
        for zone in site["zones"]:
            zone_wkt = polygon_to_wkt(zone["polygon"])
            
            cur.execute("""
                INSERT INTO functional_zones
                (city_site_id, zone_type, name, description, archaeological_findings, 
                 functional_inference, confidence_level, geom)
                VALUES (%s, %s, %s, %s, %s, %s, %s, %s)
            """, (
                site_id,
                zone["type"],
                zone["name"],
                f"{site['name']}的{zone['type']}功能区",
                zone["findings"],
                zone["inference"],
                zone["confidence"],
                zone_wkt
            ))
            zone_count += 1
    
    conn.commit()
    print(f"已插入 {zone_count} 个功能区")
    
    print("插入道路数据...")
    road_count = 0
    for site in all_sites:
        site_id = site["db_id"]
        
        for i, road in enumerate(site["roads"]):
            road_wkt = linestring_to_wkt(road["coords"])
            road_type = "主干道" if i < 3 else "次干道"
            width = random.uniform(5, 20) if road_type == "主干道" else random.uniform(2, 8)
            
            cur.execute("""
                INSERT INTO roads
                (city_site_id, road_name, road_type, width, description, geom)
                VALUES (%s, %s, %s, %s, %s, %s)
            """, (
                site_id,
                f"{site['name']}{'东西大街' if road['type'] == 'east_west' else '南北大街'}_{i+1}",
                road_type,
                width,
                f"{road_type}，路面宽约{width:.1f}米",
                road_wkt
            ))
            road_count += 1
    
    conn.commit()
    print(f"已插入 {road_count} 条道路")
    
    print("插入建筑基址数据...")
    building_count = 0
    for site in all_sites:
        site_id = site["db_id"]
        
        for building in site["buildings"]:
            point_wkt = point_to_wkt(building["lon"], building["lat"])
            
            cur.execute("""
                INSERT INTO building_foundations
                (city_site_id, building_type, name, area_sq_m, rooms_count, 
                 description, archaeological_findings, geom)
                VALUES (%s, %s, %s, %s, %s, %s, %s, %s)
            """, (
                site_id,
                building["type"],
                building["name"],
                building["area"],
                building["rooms"],
                f"{'大型' if building['area'] > 200 else '中型' if building['area'] > 80 else '小型'}{building['type']}基址",
                building["findings"],
                point_wkt
            ))
            building_count += 1
    
    conn.commit()
    print(f"已插入 {building_count} 座建筑基址")
    
    print("插入人口估算数据...")
    pop_count = 0
    for site in all_sites:
        site_id = site["db_id"]
        mean_pop = site["population"]
        
        for j in range(random.randint(1, 3)):
            year_offset = random.randint(-50, 50)
            estimate_year = (DYNASTIES[site["dynasty_id"] - 1][1] if site["dynasty_id"] - 1 < len(DYNASTIES) else 0) + year_offset
            
            cur.execute("""
                INSERT INTO population_estimates
                (city_site_id, estimate_year, population_min, population_max, 
                 population_mean, estimation_method, source, confidence_level)
                VALUES (%s, %s, %s, %s, %s, %s, %s, %s)
            """, (
                site_id,
                estimate_year,
                int(mean_pop * random.uniform(0.6, 0.85)),
                int(mean_pop * random.uniform(1.15, 1.5)),
                mean_pop,
                random.choice(["遗址面积估算法", "房屋数量推算法", "墓葬数量推算法", "粮食产量估算法"]),
                f"《{site['dynasty_name']}城市考古研究报告》",
                random.uniform(0.5, 0.85)
            ))
            pop_count += 1
    
    conn.commit()
    print(f"已插入 {pop_count} 条人口估算数据")
    
    print("插入历史地图元数据...")
    map_count = 0
    for site in all_sites[:10]:
        site_id = site["db_id"]
        
        for j in range(random.randint(1, 2)):
            cur.execute("""
                INSERT INTO historical_maps
                (city_site_id, dynasty_id, map_name, map_type, source, 
                 georeferenced, description)
                VALUES (%s, %s, %s, %s, %s, %s, %s)
            """, (
                site_id,
                site["dynasty_id"],
                f"{site['name']}历史地图{j+1}",
                random.choice(["古地图", "考古实测图", "复原图"]),
                random.choice(["明清方志", "考古报告", "历史地理研究"]),
                True,
                f"标注{site['name']}主要建筑和街道的历史地图"
            ))
            map_count += 1
    
    conn.commit()
    print(f"已插入 {map_count} 条历史地图记录")
    
    print("插入城门数据...")
    gate_count = 0
    for site in all_sites:
        site_id = site["db_id"]
        num_gates = site["num_gates"]
        center_lon = site["center_lon"]
        center_lat = site["center_lat"]
        radius_km = site["radius_km"]
        deg_per_km = 1.0 / 111.0
        
        gate_names = ["东门", "南门", "西门", "北门", "东南门", "西南门", "西北门", "东北门",
                      "东二门", "南二门", "西二门", "北二门"]
        gate_types = ["main", "main", "main", "main", "secondary", "secondary", "secondary", "secondary",
                      "tertiary", "tertiary", "tertiary", "tertiary"]
        
        for i in range(num_gates):
            angle = (i * 2 * math.pi / num_gates) - math.pi / 2
            glon = center_lon + radius_km * deg_per_km * math.cos(angle)
            glat = center_lat + radius_km * deg_per_km * math.sin(angle)
            
            base_defense = 0.6 if gate_types[i] == "main" else 0.5 if gate_types[i] == "secondary" else 0.4
            defense_rating = base_defense + random.uniform(-0.1, 0.15)
            
            point_wkt = point_to_wkt(glon, glat)
            cur.execute("""
                INSERT INTO city_gates
                (site_id, name, gate_type, defense_rating, geom, description)
                VALUES (%s, %s, %s, %s, %s, %s)
            """, (
                site_id,
                gate_names[i] if i < len(gate_names) else f"城门{i+1}",
                gate_types[i] if i < len(gate_types) else "secondary",
                round(defense_rating, 2),
                point_wkt,
                f"{site['name']}{gate_names[i] if i < len(gate_names) else f'第{i+1}门'}，为{'主' if gate_types[i] == 'main' else '次'}要城门"
            ))
            gate_count += 1
    
    conn.commit()
    print(f"已插入 {gate_count} 座城门")
    
    print("插入土地利用变迁数据...")
    land_use_count = 0
    land_use_types = [
        ("urban", "城市建成区"),
        ("farmland", "农田"),
        ("forest", "森林"),
        ("grassland", "草地"),
        ("wetland", "湿地"),
        ("water", "水体"),
        ("wasteland", "荒地"),
        ("settlement", "居民点"),
    ]
    period_names = [
        "城市鼎盛期", "城市衰落初期", "城市废弃期", "早期农业期",
        "中期农业期", "近古时期", "近代时期", "现代时期",
    ]
    
    for site in all_sites:
        site_id = site["db_id"]
        base_area = site["area_sq_km"]
        dynasty_idx = site["dynasty_id"] - 1
        seed = site_id * 0.1
        
        for p_idx, period_name in enumerate(period_names):
            t = p_idx / (len(period_names) - 1)
            urban_decay = max(0.05, 1.0 - t * 0.75)
            farmland_growth = min(0.6, 0.1 + t * 0.5 * abs(math.sin(seed * 0.1)))
            
            land_uses_pct = {}
            total_pct = 0.0
            
            urban_pct = 0.35 * urban_decay
            land_uses_pct["urban"] = urban_pct
            total_pct += urban_pct
            
            farmland_pct = farmland_growth
            land_uses_pct["farmland"] = farmland_pct
            total_pct += farmland_pct
            
            forest_pct = max(0.1, min(0.4, 0.2 + 0.1 * math.sin(t * 2 + seed)))
            land_uses_pct["forest"] = forest_pct
            total_pct += forest_pct
            
            grassland_pct = max(0.05, min(0.25, 0.15 + 0.1 * math.cos(t * 1.5 + seed)))
            land_uses_pct["grassland"] = grassland_pct
            total_pct += grassland_pct
            
            wasteland_pct = 0.3 * max(0, 1 - abs(t - 0.35) * 4) if 0.2 < t < 0.5 else 0.1
            land_uses_pct["wasteland"] = wasteland_pct
            total_pct += wasteland_pct
            
            settlement_pct = max(0.02, (t - 0.5) * 0.2) if t > 0.5 else 0.02
            land_uses_pct["settlement"] = settlement_pct
            total_pct += settlement_pct
            
            water_pct = 0.05 + 0.03 * math.sin(seed * 0.2)
            land_uses_pct["water"] = water_pct
            total_pct += water_pct
            
            wetland_pct = max(0.02, 1.0 - total_pct)
            land_uses_pct["wetland"] = wetland_pct
            
            period_year = -500 + p_idx * 300 + int(dynasty_idx * 50)
            
            for lu_type, lu_name in land_use_types:
                pct = land_uses_pct.get(lu_type, 0) * 100
                area = base_area * land_uses_pct.get(lu_type, 0)
                
                cur.execute("""
                    INSERT INTO land_use_changes
                    (site_id, period_name, period_year, land_use_type, 
                     area_km2, percentage, evidence_type, confidence, description)
                    VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s)
                """, (
                    site_id,
                    period_name,
                    period_year,
                    lu_type,
                    round(area, 4),
                    round(pct, 2),
                    random.choice(["考古地层", "孢粉分析", "历史文献", "遥感解译", "综合推断"]),
                    round(random.uniform(0.5, 0.9), 2),
                    f"{period_name}时期{lu_name}，面积约{area:.2f}平方公里"
                ))
                land_use_count += 1
    
    conn.commit()
    print(f"已插入 {land_use_count} 条土地利用变迁记录")
    
    cur.close()
    conn.close()
    
    print("\n" + "="*60)
    print("数据生成完成！")
    print(f"城市遗址: {len(all_sites)} 个")
    print(f"功能区: {zone_count} 个")
    print(f"道路: {road_count} 条")
    print(f"建筑基址: {building_count} 座")
    print(f"人口估算: {pop_count} 条")
    print(f"历史地图: {map_count} 条")
    print(f"城门: {gate_count} 座")
    print(f"土地利用记录: {land_use_count} 条")
    print("="*60)

if __name__ == "__main__":
    main()
