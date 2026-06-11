# 古代城市遗址空间结构复原与形态演化分析系统

面向考古研究所的城市遗址量化分析平台。整合**空间句法**（整合度/选择度）、**分形维数**（三算法加权融合+95%置信区间）、**Mann-Kendall趋势检验**三大模型，对从殷商到明清共50个古代城市遗址进行空间结构复原与形态演化分析。

---

## 一、系统架构

```
┌──────────────┐          ┌──────────────────┐          ┌────────────────────┐
│  前端 Nginx  │  :80     │  Rust Actix-web  │  :5432   │ PostgreSQL + PostGIS│
│  Leaflet +   │ ───────▶ │  空间句法/分形/   │ ───────▶ │  GIST空间索引       │
│  Canvas      │          │  MK趋势检验       │          │  13张表              │
│  (Gzip压缩)  │ ◀─────── │  Tracing日志      │ ◀─────── │  21朝代预置数据       │
└──────────────┘          │  Prometheus指标    │          └────────────────────┘
   ↑:8080/metrics         └──────────────────┘                  ▲
   监控接入                                                         │
                                                                   │
                                                          ┌────────┴─────────┐
                                                          │  数据模拟器       │
                                                          │  Python + psycopg2│
                                                          │  50城市 / 多参数  │
                                                          └──────────────────┘
```

### 模块职责

| 模块 | 目录 | 核心文件 | 职责 |
|------|------|----------|------|
| 后端服务 | `backend/` | [main.rs](backend/src/main.rs) | HTTP服务、Tracing日志、Prometheus指标 |
| 遗址数据加载 | `backend/src` | [city_loader.rs](backend/src/city_loader.rs) | 10个GET端点：朝代/遗址/功能区/道路/建筑/人口 |
| 形态分析引擎 | `backend/src` | [morphology_analyzer.rs](backend/src/morphology_analyzer.rs) | 空间句法+分形维数计算、道路级句法入库 |
| 演化趋势检测 | `backend/src` | [evolution_detector.rs](backend/src/evolution_detector.rs) | Mann-Kendall检验、Sen斜率、双遗址对比 |
| 空间句法 | `backend/src` | [spatial_syntax.rs](backend/src/spatial_syntax.rs) | Brandes介数算法、分块计算、u32紧凑邻接表、空间网格索引 |
| 分形维数 | `backend/src` | [fractal.rs](backend/src/fractal.rs) | 盒计数+周长面积+分规法三算法融合、Bootstrap 500次置信区间 |
| MK检验 | `backend/src` | [mann_kendall.rs](backend/src/mann_kendall.rs) | MK统计量、方差修正、Sen斜率、季节性MK |
| 算法参数 | `backend/src` | [config.rs](backend/src/config.rs) | 所有算法阈值、采样数、分块大小等集中配置 |
| 前端地图 | `frontend/js` | [city_map.js](frontend/js/city_map.js) | Leaflet+Canvas、LOD四级、视口裁剪、建筑聚合、三视图切换 |
| 形态面板 | `frontend/js` | [morphology_panel.js](frontend/js/morphology_panel.js) | 分析结果展示、置信区间+数据质量标签 |
| 趋势分析 | `frontend/js` | [trend.js](frontend/js/trend.js) | Canvas手绘折线图、MK结果展示 |
| 时间轴 | `frontend/js` | [timeline.js](frontend/js/timeline.js) | 21朝代可拖动、自动播放 |
| 数据库 | `database/` | [init.sql](database/init.sql) | 13表+GIST空间索引+21朝代+触发器 |
| 数据模拟器 | `scripts/` | [generate_data.py](scripts/generate_data.py) | 50城市遗址考古数据生成 |
| 部署 | 项目根 | [docker-compose.yml](docker-compose.yml) | 4服务：PostGIS/Backend/Frontend/Simulator |

---

## 二、快速部署

### 2.1 依赖

- Docker Engine ≥ 24.0
- Docker Compose ≥ 2.20

### 2.2 一键启动

```bash
# 1. 启动核心服务（PostGIS + Rust后端 + Nginx前端）
docker compose up -d

# 2. 等待PostGIS就绪后，生成50个城市遗址模拟数据
docker compose --profile data up simulator

# 3. 访问
#   前端:   http://localhost
#   API:    http://localhost:8080/api/health
#   指标:   http://localhost:8080/metrics
```

### 2.3 服务端口

| 服务 | 端口 | 说明 |
|------|------|------|
| frontend (Nginx) | :80 | 前端静态资源 + `/api/` 反向代理（Gzip+缓存） |
| backend (Rust) | :8080 | 16个REST API + Prometheus `/metrics` |
| postgis | :5432 | PostgreSQL 15 + PostGIS 3.4 |

### 2.4 停止与清理

```bash
# 停止所有服务
docker compose down

# 清理模拟数据容器（不会删数据库）
docker compose --profile data down

# 彻底清除数据库卷
docker compose down -v
```

---

## 三、城市遗址数据模拟器

### 3.1 功能

基于朝代演化规律生成考古数据，朝代越晚：
- 城墙形状越规则（`irregularity`从0.5递减至0.08）
- 道路网格越规整（`regularity`从0.3递增至0.9）
- 道路条数、功能区数、建筑基址数线性增长
- 人口基数按朝代索引线性增长

生成的数据包含：
- **城墙多边形**（6~12边不规则形状）
- **道路网络**（网格型，含主干道/次干道，带宽度）
- **功能区**（8种类型，含考古发现描述+功能推断+置信度）
- **建筑基址**（8种类型，含面积/房间数/出土文物）
- **人口估算**（多估算方法+置信区间）
- **历史地图元数据**

### 3.2 环境变量配置

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `DATABASE_URL` | `postgres://postgres:postgres@postgis:5432/ancient_city` | 数据库连接串 |
| `SEED` | `42` | 随机种子，相同种子产生完全相同数据 |
| `DYNASTY_FILTER` | `""` | 仅生成包含该关键字的朝代数据，如 `"唐"`、`"汉"` |

### 3.3 使用示例

```bash
# 生成全部50个城市遗址（21个朝代）
docker compose --profile data up simulator

# 仅生成唐代城市
DYNASTY_FILTER="唐" docker compose --profile data up simulator

# 使用不同种子生成变体
SEED=99 docker compose --profile data up simulator

# 本地直接运行（需要PostGIS在localhost:5432）
cd scripts && pip install psycopg2-binary
DATABASE_URL="postgres://postgres:postgres@localhost:5432/ancient_city" python generate_data.py
```

### 3.4 输出示例

```
============================================================
古代城市遗址数据模拟器
数据库: postgis:5432/ancient_city
随机种子: 42
============================================================
连接数据库...
获取朝代列表...
生成城市遗址数据...
共生成 55 个城市遗址
插入城市遗址数据...
已插入 55 个城市遗址
插入功能区数据...
已插入 332 个功能区
插入道路数据...
已插入 913 条道路
插入建筑基址数据...
已插入 1524 座建筑基址
插入人口估算数据...
已插入 127 条人口估算数据
```

---

## 四、监控与可观测性

### 4.1 Tracing 日志

后端采用 `tracing` + `tracing-subscriber` + `tracing-actix-web`，结构化输出每个请求的：
- HTTP方法、路径、状态码
- 请求耗时（秒）
- 线程ID、文件名、行号
- 业务级span（形态分析、MK检验等）

日志级别通过环境变量控制：

```bash
# 仅错误
RUST_LOG=error docker compose up -d backend

# 调试模式
RUST_LOG=debug,ancient_city_morphology=trace docker compose up -d backend
```

### 4.2 Prometheus 指标

`GET /metrics` 暴露以下指标：

| 指标名 | 类型 | 说明 |
|--------|------|------|
| `ancient_city_http_requests_total` | Counter | 请求总数，按 method/endpoint/status 分标签 |
| `ancient_city_http_request_duration_seconds` | Histogram | 请求耗时分布（10个bucket） |
| `ancient_city_syntax_compute_total` | Counter | 空间句法计算次数 |
| `ancient_city_fractal_compute_total` | Counter | 分形维数计算次数 |
| `ancient_city_mk_test_total` | Counter | Mann-Kendall检验次数 |
| `ancient_city_db_query_total` | Counter | 数据库查询次数 |
| `process_*` | Gauge/Histogram | Prometheus process指标（CPU/内存/FD） |

Prometheus 抓取配置示例：

```yaml
scrape_configs:
  - job_name: ancient-city
    static_configs:
      - targets: ['localhost:8080']
```

---

## 五、PostGIS 空间索引配置

### 5.1 空间索引

在 [init.sql](database/init.sql) 中，所有含几何字段的表均已创建 GIST 索引：

| 表 | 索引 | 几何类型 |
|----|------|----------|
| `city_sites.geom` | `idx_city_sites_geom` | Polygon（城墙范围） |
| `functional_zones.geom` | `idx_functional_zones_geom` | Polygon（功能区） |
| `roads.geom` | `idx_roads_geom` | LineString（道路） |
| `building_foundations.geom` | `idx_buildings_geom` | Point（建筑基址） |
| `road_nodes.geom` | `idx_road_nodes_geom` | Point（道路节点） |
| `historical_maps.bbox_geom` | `idx_historical_maps_bbox` | Polygon（地图范围） |

### 5.2 PostgreSQL 参数调优

在 [postgresql.conf](database/postgresql.conf) 或 docker-compose `command` 中配置：

| 参数 | 值 | 说明 |
|------|----|------|
| `shared_buffers` | 256MB | 共享缓冲区，建议物理内存1/4 |
| `effective_cache_size` | 768MB | 优化器估算缓存，建议内存3/4 |
| `work_mem` | 64MB | 排序/哈希操作内存 |
| `maintenance_work_mem` | 64MB | VACUUM/索引构建内存 |
| `min_wal_size/max_wal_size` | 2GB/8GB | WAL日志大小 |
| `random_page_cost` | 1.1 | SSD下调此值 |
| `effective_io_concurrency` | 200 | SSD随机IO并发 |
| `log_min_duration_statement` | 500ms | 慢查询日志阈值 |
| `autovacuum_vacuum_scale_factor` | 0.05 | 空间数据更新频繁，降低触发阈值 |

### 5.3 空间分析常用SQL示例

```sql
-- 某遗址内所有建筑基址
SELECT bf.* FROM building_foundations bf
JOIN city_sites cs ON ST_Contains(cs.geom, bf.geom)
WHERE cs.name = '隋唐长安城';

-- 距离某道路50米内的功能区
SELECT fz.name, fz.zone_type
FROM functional_zones fz
JOIN roads r ON ST_DWithin(r.geom::geography, fz.geom::geography, 50)
WHERE r.road_name LIKE '%朱雀大街%';

-- 功能区面积TOP10
SELECT name, zone_type, ST_Area(geom::geography)/1e6 AS area_sq_km
FROM functional_zones
ORDER BY area_sq_km DESC LIMIT 10;
```

---

## 六、前端 Gzip 与缓存

Nginx 配置 [nginx.conf](nginx.conf) 已启用：

- **Gzip 压缩**：level 6，>1KB 触发，覆盖 CSS/JS/JSON/XML/SVG/字体
- **静态资源缓存**：CSS/JS/字体/图标 30 天 `Cache-Control: public, immutable`
- **API代理**：75s连接超时、300s读取超时（形态大计算）

---

## 七、API 端点

| Method | Path | 模块 | 说明 |
|--------|------|------|------|
| GET | `/api/health` | city_loader | 健康检查 |
| GET | `/api/dynasties` | city_loader | 朝代列表 |
| GET | `/api/sites` | city_loader | 全部城市遗址 |
| GET | `/api/sites/{id}` | city_loader | 单遗址详情 |
| GET | `/api/sites/dynasty/{id}` | city_loader | 按朝代筛选遗址 |
| GET | `/api/zones/{site_id}` | city_loader | 功能区 |
| GET | `/api/roads/{site_id}` | city_loader | 道路 |
| GET | `/api/buildings/{site_id}` | city_loader | 建筑基址 |
| GET | `/api/population/{site_id}` | city_loader | 人口估算 |
| GET | `/api/morphology/{site_id}` | morphology_analyzer | 形态分析结果 |
| POST | `/api/morphology/analyze/{site_id}` | morphology_analyzer | 触发形态计算 |
| GET | `/api/syntax/roads/{site_id}` | morphology_analyzer | 道路级空间句法 |
| POST | `/api/trends/analyze` | evolution_detector | MK趋势分析 |
| GET | `/api/trends` | evolution_detector | 历史分析结果 |
| POST | `/api/compare` | evolution_detector | 双遗址对比 |
| GET | `/metrics` | main | Prometheus指标 |

---

## 八、本地开发

### 8.1 Rust 后端

```bash
cd backend
export DATABASE_URL="postgres://postgres:postgres@localhost:5432/ancient_city"
cargo run          # 开发模式
cargo build --release  # 生产构建
```

### 8.2 前端

直接用浏览器打开 `frontend/index.html` 或本地起静态服务：

```bash
cd frontend && python -m http.server 3000
```

配置 API 地址在 [config.js](frontend/js/config.js) 的 `API_BASE_URL`。
