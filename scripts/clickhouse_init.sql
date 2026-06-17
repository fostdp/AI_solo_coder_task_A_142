-- ============================================================
-- 古代司南磁石指向精度仿真与地磁场重建系统
-- ClickHouse 数据库初始化脚本
-- ============================================================

-- 创建数据库
CREATE DATABASE IF NOT EXISTS sinan_db
COMMENT '司南磁石指向精度仿真数据库';

USE sinan_db;

-- ============================================================
-- 1. 司南传感器数据表 - 存储实时上报的传感器数据
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
PARTITION BY toYYYYMM(timestamp)
ORDER BY (device_id, timestamp)
TTL timestamp + INTERVAL 1 YEAR
SETTINGS index_granularity = 8192;

-- 索引：按时间查询
CREATE INDEX IF NOT EXISTS idx_sinan_timestamp ON sinan_sensor_data (timestamp) TYPE minmax GRANULARITY 4;

-- 索引：按设备查询
CREATE INDEX IF NOT EXISTS idx_sinan_device ON sinan_sensor_data (device_id) TYPE set(0) GRANULARITY 4;

-- ============================================================
-- 2. 地磁场重建数据表 - 存储CALS10K模型计算结果
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
-- 3. 指向精度仿真结果表 - 存储微磁学仿真结果
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
-- 4. 告警事件表 - 存储指向偏差超阈值告警
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
    message String COMMENT '告警消息',
    mqtt_topic String COMMENT 'MQTT主题',
    mqtt_published Bool DEFAULT false COMMENT 'MQTT是否已推送'
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(timestamp)
ORDER BY (alert_level, timestamp)
TTL timestamp + INTERVAL 2 YEAR
SETTINGS index_granularity = 8192;

-- ============================================================
-- 5. 考古地磁数据表 - 存储考古地磁实测数据
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
-- 6. 设备信息表
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
    created_at DateTime DEFAULT now()
)
ENGINE = ReplacingMergeTree()
ORDER BY device_id
SETTINGS index_granularity = 8192;

-- ============================================================
-- 7. 视图：实时告警视图
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
-- 8. 视图：设备最新状态视图
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
    d.location_lon
FROM sinan_sensor_data s
LEFT JOIN sinan_devices d ON s.device_id = d.device_id
GROUP BY s.device_id, d.device_name, d.location_lat, d.location_lon;

-- ============================================================
-- 9. 插入示例设备数据
-- ============================================================
INSERT INTO sinan_devices (device_id, device_name, installation_date, location_lat, location_lon, magnet_material, magnet_mass, spoon_length, base_diameter) VALUES
('SINAN-001', '汉代司南原型机-1号', '2024-01-15', 34.265, 108.955, '天然磁铁矿', 750.0, 17.8, 25.0),
('SINAN-002', '汉代司南原型机-2号', '2024-01-20', 36.067, 117.123, '天然磁铁矿', 720.0, 18.2, 24.5),
('SINAN-003', '汉代司南对比实验机', '2024-02-10', 39.904, 116.407, '人造磁铁', 680.0, 17.5, 25.5);

-- ============================================================
-- 10. 插入考古地磁示例数据（汉代部分遗址）
-- ============================================================
INSERT INTO archaeomagnetic_data (site_name, location_lat, location_lon, sample_age, sample_age_error, declination, declination_error, inclination, inclination_error, intensity, intensity_error, sample_material, reference) VALUES
('汉长安城遗址', 34.265, 108.955, -100.0, 50.0, -2.5, 0.8, 56.2, 1.2, 55000.0, 3000.0, 'brick', '考古地磁学报2022'),
('洛阳汉魏故城', 34.667, 112.483, -50.0, 30.0, -1.8, 0.6, 55.8, 1.0, 54500.0, 2500.0, 'brick', '地球物理学报2021'),
('长沙马王堆汉墓', 28.197, 113.021, -165.0, 50.0, -3.2, 1.0, 48.5, 1.5, 52000.0, 3500.0, 'soil', '考古与文物2023'),
('西安未央宫遗址', 34.285, 108.925, -80.0, 40.0, -2.3, 0.7, 56.0, 1.1, 54800.0, 2800.0, 'brick', '考古地磁学报2022'),
('徐州狮子山汉墓', 34.221, 117.329, -154.0, 60.0, -2.8, 0.9, 52.3, 1.3, 53500.0, 3200.0, 'soil', '华夏考古2022');

-- ============================================================
-- 11. 初始化完成信息
-- ============================================================
SELECT 'ClickHouse数据库初始化完成' as status, now() as init_time;
