-- ============================================================
-- 古代司南磁石指向精度仿真与地磁场重建系统
-- ClickHouse 数据库初始化脚本 (含降采样与保留策略)
-- ============================================================

-- 创建数据库
CREATE DATABASE IF NOT EXISTS sinan_db
COMMENT '司南磁石指向精度仿真数据库';

USE sinan_db;

-- ============================================================
-- 1. 原始传感器数据表 (热数据)
-- 保留策略: 7天 + 1个月到归档存储
-- ============================================================
CREATE TABLE IF NOT EXISTS sinan_sensor_data (
    id UUID DEFAULT generateUUIDv4(),
    device_id String COMMENT '司南设备编号',
    timestamp DateTime64(3, 'UTC') DEFAULT now64() COMMENT '采集时间戳',
    magnetic_moment_x Float64 COMMENT '磁矩X分量 (A·m²)',
    magnetic_moment_y Float64 COMMENT '磁矩Y分量 (A·m²)',
    magnetic_moment_z Float64 COMMENT '磁矩Z分量 (A·m²)',
    magnetic_moment_magnitude Float64 COMMENT '磁矩大小 (A·m²)',
    remanence Float64 COMMENT '剩磁强度 (T)',
    pointing_deviation Float64 COMMENT '指向偏差 (度)',
    environment_temp Float64 COMMENT '环境温度 (°C)',
    location_lat Float64 COMMENT '纬度',
    location_lon Float64 COMMENT '经度',
    is_alert Bool DEFAULT false COMMENT '是否告警'
)
ENGINE = MergeTree()
PARTITION BY toYYYYMMDD(timestamp)
ORDER BY (device_id, timestamp)
TTL
    timestamp + INTERVAL 7 DAY TO VOLUME 'hot',
    timestamp + INTERVAL 1 MONTH TO VOLUME 'cold',
    timestamp + INTERVAL 3 MONTH DELETE
SETTINGS
    index_granularity = 8192,
    storage_policy = 'default';

-- 索引：按时间查询
CREATE INDEX IF NOT EXISTS idx_sinan_timestamp ON sinan_sensor_data (timestamp) TYPE minmax GRANULARITY 4;

-- 索引：按设备查询
CREATE INDEX IF NOT EXISTS idx_sinan_device ON sinan_sensor_data (device_id) TYPE set(0) GRANULARITY 4;

-- 索引：告警快速查询
CREATE INDEX IF NOT EXISTS idx_sinan_alert ON sinan_sensor_data (is_alert) TYPE set(0) GRANULARITY 4;

-- ============================================================
-- 2. 1分钟粒度降采样表 (近7天)
-- ============================================================
CREATE TABLE IF NOT EXISTS sinan_sensor_data_1min (
    device_id String COMMENT '司南设备编号',
    timestamp DateTime64(0, 'UTC') COMMENT '统计开始时间',
    count UInt32 COMMENT '数据点数',
    avg_magnetic_moment_magnitude Float64 COMMENT '平均磁矩大小',
    min_magnetic_moment_magnitude Float64 COMMENT '最小磁矩大小',
    max_magnetic_moment_magnitude Float64 COMMENT '最大磁矩大小',
    avg_remanence Float64 COMMENT '平均剩磁强度',
    min_remanence Float64 COMMENT '最小剩磁强度',
    max_remanence Float64 COMMENT '最大剩磁强度',
    avg_pointing_deviation Float64 COMMENT '平均指向偏差',
    min_pointing_deviation Float64 COMMENT '最小指向偏差',
    max_pointing_deviation Float64 COMMENT '最大指向偏差',
    p95_pointing_deviation Float64 COMMENT '95分位指向偏差',
    avg_environment_temp Float64 COMMENT '平均环境温度',
    alert_count UInt32 COMMENT '告警次数'
)
ENGINE = SummingMergeTree()
PARTITION BY toYYYYMM(timestamp)
ORDER BY (device_id, timestamp)
TTL timestamp + INTERVAL 7 DAY
SETTINGS index_granularity = 8192;

-- ============================================================
-- 3. 1小时粒度降采样表 (近30天)
-- ============================================================
CREATE TABLE IF NOT EXISTS sinan_sensor_data_1hour (
    device_id String COMMENT '司南设备编号',
    timestamp DateTime64(0, 'UTC') COMMENT '统计开始时间',
    count UInt32 COMMENT '数据点数',
    avg_magnetic_moment_magnitude Float64 COMMENT '平均磁矩大小',
    min_magnetic_moment_magnitude Float64 COMMENT '最小磁矩大小',
    max_magnetic_moment_magnitude Float64 COMMENT '最大磁矩大小',
    avg_remanence Float64 COMMENT '平均剩磁强度',
    min_remanence Float64 COMMENT '最小剩磁强度',
    max_remanence Float64 COMMENT '最大剩磁强度',
    avg_pointing_deviation Float64 COMMENT '平均指向偏差',
    min_pointing_deviation Float64 COMMENT '最小指向偏差',
    max_pointing_deviation Float64 COMMENT '最大指向偏差',
    p95_pointing_deviation Float64 COMMENT '95分位指向偏差',
    avg_environment_temp Float64 COMMENT '平均环境温度',
    alert_count UInt32 COMMENT '告警次数'
)
ENGINE = SummingMergeTree()
PARTITION BY toYYYYMM(timestamp)
ORDER BY (device_id, timestamp)
TTL timestamp + INTERVAL 30 DAY
SETTINGS index_granularity = 8192;

-- ============================================================
-- 4. 1天粒度降采样表 (永久保留)
-- ============================================================
CREATE TABLE IF NOT EXISTS sinan_sensor_data_1day (
    device_id String COMMENT '司南设备编号',
    timestamp Date COMMENT '统计日期',
    count UInt32 COMMENT '数据点数',
    avg_magnetic_moment_magnitude Float64 COMMENT '平均磁矩大小',
    min_magnetic_moment_magnitude Float64 COMMENT '最小磁矩大小',
    max_magnetic_moment_magnitude Float64 COMMENT '最大磁矩大小',
    avg_remanence Float64 COMMENT '平均剩磁强度',
    min_remanence Float64 COMMENT '最小剩磁强度',
    max_remanence Float64 COMMENT '最大剩磁强度',
    avg_pointing_deviation Float64 COMMENT '平均指向偏差',
    min_pointing_deviation Float64 COMMENT '最小指向偏差',
    max_pointing_deviation Float64 COMMENT '最大指向偏差',
    p95_pointing_deviation Float64 COMMENT '95分位指向偏差',
    avg_environment_temp Float64 COMMENT '平均环境温度',
    min_environment_temp Float64 COMMENT '最低环境温度',
    max_environment_temp Float64 COMMENT '最高环境温度',
    alert_count UInt32 COMMENT '告警次数',
    alert_acknowledged UInt32 COMMENT '已确认告警数'
)
ENGINE = SummingMergeTree()
PARTITION BY toYYYYMM(timestamp)
ORDER BY (device_id, timestamp)
TTL timestamp + INTERVAL 10 YEAR
SETTINGS index_granularity = 8192;

-- ============================================================
-- 5. 物化视图：原始数据 -> 1分钟降采样
-- ============================================================
CREATE MATERIALIZED VIEW IF NOT EXISTS mv_sensor_1min
TO sinan_sensor_data_1min
AS SELECT
    device_id,
    toStartOfMinute(timestamp) AS timestamp,
    count() AS count,
    avg(magnetic_moment_magnitude) AS avg_magnetic_moment_magnitude,
    min(magnetic_moment_magnitude) AS min_magnetic_moment_magnitude,
    max(magnetic_moment_magnitude) AS max_magnetic_moment_magnitude,
    avg(remanence) AS avg_remanence,
    min(remanence) AS min_remanence,
    max(remanence) AS max_remanence,
    avg(pointing_deviation) AS avg_pointing_deviation,
    min(pointing_deviation) AS min_pointing_deviation,
    max(pointing_deviation) AS max_pointing_deviation,
    quantile(0.95)(pointing_deviation) AS p95_pointing_deviation,
    avg(environment_temp) AS avg_environment_temp,
    countIf(is_alert) AS alert_count
FROM sinan_sensor_data
GROUP BY device_id, timestamp;

-- ============================================================
-- 6. 物化视图：1分钟 -> 1小时降采样
-- ============================================================
CREATE MATERIALIZED VIEW IF NOT EXISTS mv_sensor_1hour
TO sinan_sensor_data_1hour
AS SELECT
    device_id,
    toStartOfHour(timestamp) AS timestamp,
    sum(count) AS count,
    avg(avg_magnetic_moment_magnitude) AS avg_magnetic_moment_magnitude,
    min(min_magnetic_moment_magnitude) AS min_magnetic_moment_magnitude,
    max(max_magnetic_moment_magnitude) AS max_magnetic_moment_magnitude,
    avg(avg_remanence) AS avg_remanence,
    min(min_remanence) AS min_remanence,
    max(max_remanence) AS max_remanence,
    avg(avg_pointing_deviation) AS avg_pointing_deviation,
    min(min_pointing_deviation) AS min_pointing_deviation,
    max(max_pointing_deviation) AS max_pointing_deviation,
    max(p95_pointing_deviation) AS p95_pointing_deviation,
    avg(avg_environment_temp) AS avg_environment_temp,
    sum(alert_count) AS alert_count
FROM sinan_sensor_data_1min
GROUP BY device_id, timestamp;

-- ============================================================
-- 7. 物化视图：1小时 -> 1天降采样
-- ============================================================
CREATE MATERIALIZED VIEW IF NOT EXISTS mv_sensor_1day
TO sinan_sensor_data_1day
AS SELECT
    device_id,
    toDate(timestamp) AS timestamp,
    sum(count) AS count,
    avg(avg_magnetic_moment_magnitude) AS avg_magnetic_moment_magnitude,
    min(min_magnetic_moment_magnitude) AS min_magnetic_moment_magnitude,
    max(max_magnetic_moment_magnitude) AS max_magnetic_moment_magnitude,
    avg(avg_remanence) AS avg_remanence,
    min(min_remanence) AS min_remanence,
    max(max_remanence) AS max_remanence,
    avg(avg_pointing_deviation) AS avg_pointing_deviation,
    min(min_pointing_deviation) AS min_pointing_deviation,
    max(max_pointing_deviation) AS max_pointing_deviation,
    max(p95_pointing_deviation) AS p95_pointing_deviation,
    avg(avg_environment_temp) AS avg_environment_temp,
    min(avg_environment_temp) AS min_environment_temp,
    max(avg_environment_temp) AS max_environment_temp,
    sum(alert_count) AS alert_count,
    0 AS alert_acknowledged
FROM sinan_sensor_data_1hour
GROUP BY device_id, timestamp;

-- ============================================================
-- 8. 地磁场重建数据表
-- ============================================================
CREATE TABLE IF NOT EXISTS geomagnetic_field_data (
    id UUID DEFAULT generateUUIDv4(),
    timestamp DateTime64(3, 'UTC') DEFAULT now64() COMMENT '计算时间',
    target_year Float64 COMMENT '目标年份 (公元年，负数为公元前)',
    location_lat Float64 COMMENT '纬度',
    location_lon Float64 COMMENT '经度',
    field_intensity Float64 COMMENT '地磁场强度 (nT)',
    declination Float64 COMMENT '磁偏角 (度)',
    inclination Float64 COMMENT '磁倾角 (度)',
    bx Float64 COMMENT '地磁场X分量 (nT)',
    by Float64 COMMENT '地磁场Y分量 (nT)',
    bz Float64 COMMENT '地磁场Z分量 (nT)',
    model_source String DEFAULT 'CALS10K' COMMENT '模型来源'
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(timestamp)
ORDER BY (target_year, location_lat, location_lon, timestamp)
TTL timestamp + INTERVAL 5 YEAR
SETTINGS index_granularity = 8192;

-- ============================================================
-- 9. 指向精度仿真结果表
-- ============================================================
CREATE TABLE IF NOT EXISTS pointing_simulation_results (
    id UUID DEFAULT generateUUIDv4(),
    timestamp DateTime64(3, 'UTC') DEFAULT now64(),
    device_id String COMMENT '司南设备编号',
    simulation_id String COMMENT '仿真批次ID',
    target_year Float64 COMMENT '仿真年份',
    location_lat Float64 COMMENT '纬度',
    location_lon Float64 COMMENT '经度',
    expected_azimuth Float64 COMMENT '理论方位角 (度)',
    simulated_azimuth Float64 COMMENT '仿真方位角 (度)',
    pointing_accuracy Float64 COMMENT '指向精度 (度)',
    magnetic_moment_magnitude Float64 COMMENT '磁矩大小 (A·m²)',
    remanence Float64 COMMENT '剩磁强度 (T)',
    temperature Float64 COMMENT '温度 (°C)',
    friction_coefficient Float64 COMMENT '摩擦系数',
    demagnetization_factor Float64 COMMENT '退磁因子',
    anisotropy_constant Float64 COMMENT '磁各向异性常数 (J/m³)',
    model_parameters String COMMENT '模型参数JSON'
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(timestamp)
ORDER BY (device_id, simulation_id, timestamp)
TTL timestamp + INTERVAL 5 YEAR
SETTINGS index_granularity = 8192;

-- ============================================================
-- 10. 告警事件表
-- ============================================================
CREATE TABLE IF NOT EXISTS alert_events (
    id UUID DEFAULT generateUUIDv4(),
    timestamp DateTime64(3, 'UTC') DEFAULT now64(),
    device_id String COMMENT '司南设备编号',
    alert_type String COMMENT '告警类型：POINTING_DEVIATION',
    alert_level String COMMENT '告警级别：WARNING/CRITICAL',
    pointing_deviation Float64 COMMENT '指向偏差 (度)',
    threshold Float64 DEFAULT 5.0 COMMENT '告警阈值 (度)',
    sensor_data_id UUID COMMENT '关联传感器数据ID',
    is_acknowledged Bool DEFAULT false COMMENT '是否已确认',
    acknowledged_at DateTime64(3, 'UTC') COMMENT '确认时间',
    acknowledged_by String COMMENT '确认人',
    message String COMMENT '告警消息',
    mqtt_topic String COMMENT 'MQTT主题',
    mqtt_published Bool DEFAULT false COMMENT 'MQTT是否已推送',
    mqtt_published_at DateTime64(3, 'UTC') COMMENT 'MQTT推送时间'
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(timestamp)
ORDER BY (alert_level, timestamp)
TTL timestamp + INTERVAL 2 YEAR
SETTINGS index_granularity = 8192;

-- ============================================================
-- 11. 告警事件日汇总表
-- ============================================================
CREATE TABLE IF NOT EXISTS alert_events_daily (
    timestamp Date COMMENT '统计日期',
    device_id String COMMENT '司南设备编号',
    alert_level String COMMENT '告警级别',
    total_count UInt32 COMMENT '总告警数',
    acknowledged_count UInt32 COMMENT '已确认数',
    avg_pointing_deviation Float64 COMMENT '平均偏差',
    max_pointing_deviation Float64 COMMENT '最大偏差'
)
ENGINE = SummingMergeTree()
PARTITION BY toYYYYMM(timestamp)
ORDER BY (timestamp, device_id, alert_level)
TTL timestamp + INTERVAL 1 YEAR
SETTINGS index_granularity = 8192;

-- ============================================================
-- 12. 物化视图：告警事件日汇总
-- ============================================================
CREATE MATERIALIZED VIEW IF NOT EXISTS mv_alerts_daily
TO alert_events_daily
AS SELECT
    toDate(timestamp) AS timestamp,
    device_id,
    alert_level,
    count() AS total_count,
    countIf(is_acknowledged) AS acknowledged_count,
    avg(pointing_deviation) AS avg_pointing_deviation,
    max(pointing_deviation) AS max_pointing_deviation
FROM alert_events
GROUP BY timestamp, device_id, alert_level;

-- ============================================================
-- 13. 考古地磁数据表
-- ============================================================
CREATE TABLE IF NOT EXISTS archaeomagnetic_data (
    id UUID DEFAULT generateUUIDv4(),
    site_name String COMMENT '考古遗址名称',
    location_lat Float64 COMMENT '纬度',
    location_lon Float64 COMMENT '经度',
    sample_age Float64 COMMENT '样本年代 (公元年)',
    sample_age_error Float64 COMMENT '年代误差 (年)',
    declination Float64 COMMENT '磁偏角 (度)',
    declination_error Float64 COMMENT '磁偏角误差 (度)',
    inclination Float64 COMMENT '磁倾角 (度)',
    inclination_error Float64 COMMENT '磁倾角误差 (度)',
    intensity Float64 COMMENT '磁场强度 (nT)',
    intensity_error Float64 COMMENT '磁场强度误差 (nT)',
    sample_material String COMMENT '样本材料：brick/soil/ceramic',
    reference String COMMENT '参考文献'
)
ENGINE = MergeTree()
ORDER BY (site_name, sample_age)
SETTINGS index_granularity = 8192;

-- ============================================================
-- 14. 设备信息表
-- ============================================================
CREATE TABLE IF NOT EXISTS sinan_devices (
    device_id String COMMENT '司南设备编号',
    device_name String COMMENT '设备名称',
    installation_date Date COMMENT '安装日期',
    location_lat Float64 COMMENT '安装纬度',
    location_lon Float64 COMMENT '安装经度',
    magnet_material String COMMENT '磁石材料',
    magnet_mass Float64 COMMENT '磁石质量 (g)',
    spoon_length Float64 COMMENT '勺长 (cm)',
    base_diameter Float64 COMMENT '底盘直径 (cm)',
    is_active Bool DEFAULT true,
    created_at DateTime DEFAULT now(),
    updated_at DateTime DEFAULT now()
)
ENGINE = ReplacingMergeTree(updated_at)
ORDER BY device_id
SETTINGS index_granularity = 8192;

-- ============================================================
-- 15. 视图：实时告警视图
-- ============================================================
CREATE VIEW IF NOT EXISTS active_alerts
AS
SELECT
    a.timestamp,
    a.device_id,
    a.alert_type,
    a.alert_level,
    a.pointing_deviation,
    a.threshold,
    a.message,
    d.location_lat,
    d.location_lon,
    d.device_name
FROM alert_events a
LEFT JOIN sinan_devices d ON a.device_id = d.device_id
WHERE a.is_acknowledged = false
ORDER BY a.timestamp DESC;

-- ============================================================
-- 16. 视图：设备最新状态视图
-- ============================================================
CREATE VIEW IF NOT EXISTS device_latest_status
AS
SELECT
    s.device_id,
    d.device_name,
    max(s.timestamp) as last_report_time,
    argMax(s.pointing_deviation, s.timestamp) as latest_deviation,
    argMax(s.remanence, s.timestamp) as latest_remanence,
    argMax(s.environment_temp, s.timestamp) as latest_temp,
    argMax(s.is_alert, s.timestamp) as is_alerting,
    d.location_lat,
    d.location_lon,
    d.magnet_material
FROM sinan_sensor_data s
LEFT JOIN sinan_devices d ON s.device_id = d.device_id
GROUP BY s.device_id, d.device_name, d.location_lat, d.location_lon, d.magnet_material;

-- ============================================================
-- 17. 视图：24小时统计视图
-- ============================================================
CREATE VIEW IF NOT EXISTS device_24h_stats
AS
SELECT
    device_id,
    count() as data_points,
    avg(pointing_deviation) as avg_deviation,
    max(pointing_deviation) as max_deviation,
    quantile(0.95)(pointing_deviation) as p95_deviation,
    countIf(is_alert) as alert_count
FROM sinan_sensor_data
WHERE timestamp > now() - INTERVAL 24 HOUR
GROUP BY device_id;

-- ============================================================
-- 18. 插入示例设备数据
-- ============================================================
INSERT INTO sinan_devices (device_id, device_name, installation_date, location_lat, location_lon, magnet_material, magnet_mass, spoon_length, base_diameter) VALUES
('SINAN-001', '汉代司南原型机-1号', '2024-01-15', 34.265, 108.955, '天然磁铁矿', 750.0, 17.8, 25.0),
('SINAN-002', '汉代司南原型机-2号', '2024-01-20', 36.067, 117.123, '天然磁铁矿', 720.0, 18.2, 24.5),
('SINAN-003', '汉代司南对比实验机', '2024-02-10', 39.904, 116.407, '人造磁铁', 680.0, 17.5, 25.5),
('SINAN-004', '钕铁硼现代司南', '2024-03-01', 31.230, 121.474, '钕铁硼', 500.0, 17.0, 24.0),
('SINAN-005', '铝镍钴实验司南', '2024-03-15', 22.543, 114.058, '铝镍钴', 600.0, 17.5, 24.5);

-- ============================================================
-- 19. 插入考古地磁示例数据（汉代部分遗址）
-- ============================================================
INSERT INTO archaeomagnetic_data (site_name, location_lat, location_lon, sample_age, sample_age_error, declination, declination_error, inclination, inclination_error, intensity, intensity_error, sample_material, reference) VALUES
('汉长安城遗址', 34.265, 108.955, -100.0, 50.0, -2.5, 0.8, 56.2, 1.2, 55000.0, 3000.0, 'brick', '考古地磁学报2022'),
('洛阳汉魏故城', 34.667, 112.483, -50.0, 30.0, -1.8, 0.6, 55.8, 1.0, 54500.0, 2500.0, 'brick', '地球物理学报2021'),
('长沙马王堆汉墓', 28.197, 113.021, -165.0, 50.0, -3.2, 1.0, 48.5, 1.5, 52000.0, 3500.0, 'soil', '考古与文物2023'),
('西安未央宫遗址', 34.285, 108.925, -80.0, 40.0, -2.3, 0.7, 56.0, 1.1, 54800.0, 2800.0, 'brick', '考古地磁学报2022'),
('徐州狮子山汉墓', 34.221, 117.329, -154.0, 60.0, -2.8, 0.9, 52.3, 1.3, 53500.0, 3200.0, 'soil', '华夏考古2022'),
('满城汉墓', 38.958, 115.319, -120.0, 45.0, -2.0, 0.8, 54.8, 1.1, 54200.0, 2900.0, 'ceramic', '文物2021'),
('南越王墓', 23.132, 113.266, -130.0, 55.0, -3.5, 1.1, 38.5, 1.6, 48500.0, 4000.0, 'soil', '考古2022'),
('龟山汉墓', 34.256, 117.186, -140.0, 50.0, -2.7, 0.9, 51.8, 1.3, 53800.0, 3100.0, 'brick', '考古与文物2021'),
('老山汉墓', 39.915, 116.240, -100.0, 40.0, -1.5, 0.7, 56.5, 1.1, 55500.0, 2700.0, 'soil', '北京文物2023'),
('洛庄汉墓', 36.795, 117.453, -170.0, 60.0, -2.2, 1.0, 53.5, 1.4, 54000.0, 3300.0, 'brick', '华夏考古2021'),
('双乳山汉墓', 36.383, 116.584, -150.0, 55.0, -2.4, 0.9, 54.0, 1.3, 53900.0, 3000.0, 'soil', '考古学报2022'),
('银雀山汉墓', 35.057, 118.336, -140.0, 50.0, -2.6, 0.8, 53.2, 1.2, 53600.0, 2900.0, 'brick', '文物2022'),
('凤凰山汉墓', 30.345, 114.325, -160.0, 55.0, -3.0, 1.0, 49.5, 1.4, 51500.0, 3400.0, 'soil', '江汉考古2021'),
('马王堆三号墓', 28.197, 113.021, -168.0, 45.0, -3.3, 0.9, 48.8, 1.3, 51800.0, 3200.0, 'soil', '考古2023'),
('大葆台汉墓', 39.817, 116.282, -90.0, 40.0, -1.8, 0.7, 56.0, 1.0, 55200.0, 2600.0, 'brick', '北京文博2022'),
('古滇国墓葬', 24.980, 102.705, -120.0, 60.0, -4.0, 1.2, 42.5, 1.7, 46500.0, 4200.0, 'soil', '云南文物2021'),
('高句丽墓葬', 41.155, 126.023, -100.0, 50.0, -1.0, 0.8, 58.5, 1.2, 56800.0, 2800.0, 'brick', '北方文物2022'),
('南越国宫署', 23.129, 113.264, -135.0, 50.0, -3.4, 1.0, 38.2, 1.4, 48200.0, 3800.0, 'soil', '广州文博2021'),
('汉阳陵', 34.442, 108.946, -140.0, 45.0, -2.4, 0.7, 55.8, 1.1, 54600.0, 2700.0, 'brick', '考古与文物2023'),
('汉魏洛阳城', 34.667, 112.483, -180.0, 50.0, -2.0, 0.8, 55.5, 1.2, 54300.0, 2900.0, 'brick', '中原文物2022');

-- ============================================================
-- 20. 数据保留策略说明
-- ============================================================
SELECT 'ClickHouse数据库初始化完成' as status;
SELECT '' as remark;
SELECT '数据保留策略:' as ttl_info;
SELECT '  原始数据 (sinan_sensor_data): 7天热数据 + 1个月冷数据 + 3个月删除' as ttl_1;
SELECT '  1分钟汇总 (sinan_sensor_data_1min): 7天' as ttl_2;
SELECT '  1小时汇总 (sinan_sensor_data_1hour): 30天' as ttl_3;
SELECT '  1天汇总 (sinan_sensor_data_1day): 10年' as ttl_4;
SELECT '  告警事件 (alert_events): 2年' as ttl_5;
SELECT '  仿真结果 (pointing_simulation_results): 5年' as ttl_6;
SELECT '  地磁场数据 (geomagnetic_field_data): 5年' as ttl_7;
SELECT '' as remark;
SELECT '降采样链路:' as rollup_info;
SELECT '  原始数据 → 1分钟 MV → 1小时 MV → 1天 MV' as rollup_1;
SELECT now() as init_time;
