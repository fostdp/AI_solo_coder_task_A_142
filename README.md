# 古代司南磁石指向精度仿真与地磁场重建系统

## 项目概述

本系统为某科技史团队提供汉代司南复原研究的全栈技术支持。通过微磁学理论和地磁场模型，仿真计算磁石在汉代地磁场下的指向精度，并基于考古地磁数据和CALS10K模型重建汉代地磁场分布。

## 系统架构

```
┌─────────────────┐     HTTP/MQTT     ┌─────────────────┐     ClickHouse    ┌──────────────┐
│  传感器模拟器    │ ────────────────> │   Rust 后端服务   │ ────────────────> │  时序数据库   │
│  (Python)        │                   │   (axum + tokio)  │                   │              │
└─────────────────┘                   └─────────────────┘                   └──────────────┘
                                                               │
                                                               │ REST API / SSE
                                                               ▼
                                                       ┌─────────────────┐
                                                       │   前端可视化     │
                                                       │ (Three.js + Canvas)│
                                                       └─────────────────┘
```

## 技术栈

### 后端
- **语言**: Rust 1.75+
- **Web框架**: axum 0.7
- **异步运行时**: tokio 1.35
- **数据库**: ClickHouse 23.x
- **MQTT客户端**: rumqttc 0.24
- **线性代数**: nalgebra 0.32
- **序列化**: serde + serde_json

### 前端
- **3D渲染**: Three.js r160
- **图表**: Chart.js 4.x
- **样式**: 原生CSS3
- **构建**: 无，直接运行

### 数据库
- **存储引擎**: ClickHouse MergeTree
- **索引**: 跳数索引、布隆过滤器索引
- **数据保留**: TTL自动清理（365天）

### 模拟器
- **语言**: Python 3.9+
- **HTTP客户端**: requests
- **MQTT客户端**: paho-mqtt

## 核心功能

### 1. 磁指向仿真模型
基于微磁学理论，整合以下物理效应：
- **朗之万函数**: 计算平衡磁化强度
- **斯托纳-沃尔法斯模型**: 单畴颗粒开关场计算
- **退磁效应**: 椭球体退磁因子修正
- **磁各向异性**: 磁晶各向异性能量
- **热扰动**: 温度相关的随机涨落
- **摩擦阻尼**: 机械摩擦能量耗散

### 2. 地磁场重建
基于CALS10K全球地磁场模型：
- **球谐展开**: 10阶球谐系数
- **勒让德函数**: 缔合勒让德多项式计算
- **长期变**: 时间序列插值（公元前3000年 - 公元2000年）
- **考古地磁校准**: 汉代遗址实测数据校准

### 3. 告警系统
- **告警阈值**: 指向偏差 > 5°
- **告警推送**: MQTT异步推送
- **告警级别**: WARNING(5°-10°), CRITICAL(>10°)
- **告警确认**: 支持人工确认

## 目录结构

```
AI_solo_coder_task_A_142/
├── backend/                    # Rust后端
│   ├── src/
│   │   ├── main.rs            # 主程序入口
│   │   ├── config.rs          # 配置模块
│   │   ├── errors.rs          # 错误处理
│   │   ├── models.rs          # 数据模型
│   │   ├── database.rs        # 数据库操作
│   │   ├── mqtt_service.rs    # MQTT服务
│   │   ├── alert_service.rs   # 告警服务
│   │   ├── handlers.rs        # API处理器
│   │   ├── micromagnetic_simulation.rs  # 微磁学仿真
│   │   └── cals10k_model.rs   # CALS10K地磁场模型
│   ├── Cargo.toml
│   └── .env.example
├── frontend/                   # 前端应用
│   ├── index.html
│   ├── css/
│   │   └── style.css
│   └── js/
│       ├── config.js          # 配置
│       ├── data.js            # 数据服务
│       ├── sinan3d.js         # 3D司南模型
│       ├── vectorfield.js     # 矢量场渲染
│       ├── charts.js          # 图表
│       └── main.js            # 主入口
├── scripts/                    # 脚本
│   ├── clickhouse_init.sql    # 数据库初始化
│   ├── sensor_simulator.py    # 传感器模拟器
│   └── requirements.txt       # Python依赖
└── README.md
```

## 快速开始

### 1. 环境准备

#### ClickHouse
```bash
# 启动ClickHouse（使用Docker）
docker run -d \
  --name clickhouse \
  -p 8123:8123 \
  -p 9000:9000 \
  --ulimit nofile=262144:262144 \
  clickhouse/clickhouse-server:23.8
```

#### MQTT Broker（可选）
```bash
# 启动EMQX
docker run -d \
  --name emqx \
  -p 1883:1883 \
  -p 8083:8083 \
  -p 8084:8084 \
  -p 18083:18083 \
  emqx/emqx:5.3
```

### 2. 数据库初始化
```bash
# 执行初始化脚本
clickhouse-client --host localhost < scripts/clickhouse_init.sql
```

### 3. 后端启动
```bash
cd backend

# 复制并修改配置
cp .env.example .env

# 编译并运行
cargo build --release
cargo run --release
```

### 4. 前端启动
```bash
cd frontend

# 使用Python启动简单HTTP服务器
python -m http.server 8081

# 浏览器访问 http://localhost:8081
```

### 5. 传感器模拟器
```bash
cd scripts
pip install -r requirements.txt

# 单设备模式
python sensor_simulator.py --device-id SINAN-001

# 多设备模式（3台司南同时上报）
python sensor_simulator.py --multi

# 使用MQTT推送
python sensor_simulator.py --multi --use-mqtt
```

## API接口

### 传感器数据
| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/v1/sensor` | 上报传感器数据 |
| GET | `/api/v1/sensor/data` | 查询传感器数据 |
| GET | `/api/v1/sensor/latest` | 获取最新数据 |
| GET | `/api/v1/sensor/stream` | SSE实时数据流 |

### 地磁场
| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/v1/geomagnetic/field` | 计算单点地磁场 |
| POST | `/api/v1/geomagnetic/vectorfield` | 生成矢量场 |
| GET | `/api/v1/geomagnetic/secular` | 获取长期变数据 |

### 仿真
| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/v1/simulation/pointing` | 运行指向仿真 |
| GET | `/api/v1/simulation/results` | 查询仿真结果 |

### 告警
| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/v1/alerts/active` | 获取活动告警 |
| POST | `/api/v1/alerts/acknowledge` | 确认告警 |

### 系统
| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/v1/devices` | 获取设备列表 |
| GET | `/api/v1/devices/status` | 获取设备状态 |
| GET | `/api/v1/statistics` | 获取统计数据 |
| GET | `/health` | 健康检查 |

## 配置说明

### 后端配置 (.env)
```env
# ClickHouse
CLICKHOUSE_HOST=localhost
CLICKHOUSE_PORT=9000
CLICKHOUSE_USER=default
CLICKHOUSE_PASSWORD=
CLICKHOUSE_DATABASE=sinan_db

# MQTT
MQTT_ENABLED=true
MQTT_BROKER=localhost
MQTT_PORT=1883
MQTT_TOPIC=sinan/alerts

# 服务器
SERVER_HOST=0.0.0.0
SERVER_PORT=8080

# 告警
ALERT_DEVIATION_THRESHOLD=5.0
ALERT_COOLDOWN_SECONDS=300
```

### 前端配置 (js/config.js)
```javascript
const CONFIG = {
    API_BASE_URL: 'http://localhost:8080',
    THRESHOLDS: {
        WARNING: 5.0,
        CRITICAL: 10.0
    },
    UPDATE_INTERVALS: {
        DEVICES: 30000,
        ALERTS: 10000
    }
};
```

## 仿真模型参数

### 微磁学仿真参数
| 参数 | 默认值 | 说明 |
|------|--------|------|
| 磁矩大小 | 0.025 A·m² | 磁石磁偶极矩 |
| 剩磁强度 | 0.85 | 剩余磁化强度比率 |
| 温度 | 25.0 °C | 环境温度 |
| 摩擦系数 | 0.05 | 机械摩擦阻尼 |
| 退磁因子 | 0.1 | 椭球体退磁效应 |
| 各向异性常数 | 1e4 J/m³ | 磁晶各向异性能 |

### CALS10K模型参数
| 参数 | 范围 | 说明 |
|------|------|------|
| 目标年份 | -3000 ~ 2000 | 公元前3000年到公元2000年 |
| 球谐阶数 | 10 | 模型展开阶数 |
| 海拔高度 | 0 ~ 100 km | 计算点海拔 |

## 前端功能

### 3D司南视图
- Three.js渲染的真实感司南模型
- 实时方位角更新（平滑动画）
- 磁场线可视化
- 偏差告警动画效果
- 交互式视角控制

### 地磁场矢量场视图
- Canvas绘制的地磁矢量场
- 热力图显示场强分布
- 箭头显示磁场方向
- 动画粒子模拟磁场流动
- 可调节网格密度

### 数据图表视图
- 指向偏差趋势图（带阈值线）
- 各遗址地磁场强度对比
- 磁矩三分量变化曲线
- 温度-偏差散点图

### 仿真结果视图
- 仿真参数配置面板
- 实时仿真运行
- 历史仿真结果表格
- 指向精度色标显示

## MQTT告警

### 告警主题
```
sinan/alerts/{device_id}
```

### 告警消息格式
```json
{
    "id": "alert-uuid",
    "device_id": "SINAN-001",
    "alert_level": "WARNING",
    "deviation": 6.5,
    "threshold": 5.0,
    "message": "指向偏差6.5°超过阈值5.0°",
    "timestamp": "2024-01-15T10:30:00Z",
    "is_acknowledged": false
}
```

## 数据库设计

### 核心数据表

#### sinan_sensor_data (传感器数据表)
- 存储每台司南每分钟上报的传感器数据
- 按设备ID和时间戳分区
- TTL: 365天

#### geomagnetic_field_data (地磁场数据表)
- 存储计算得到的地磁场数据
- 包含磁偏角、磁倾角、场强

#### pointing_simulation_results (仿真结果表)
- 存储每次指向仿真的结果
- 包含预期方位、仿真方位、指向精度

#### alert_events (告警事件表)
- 存储所有告警事件
- 支持告警确认状态跟踪

#### archaeomagnetic_data (考古地磁数据表)
- 存储汉代遗址实测地磁数据
- 用于模型校准

#### sinan_devices (设备信息表)
- 存储司南设备元信息
- 包含设备位置、安装时间

## 性能指标

- **数据写入**: 支持1000+台设备同时上报（1分钟间隔）
- **查询响应**: 单条查询 < 100ms
- **矢量场生成**: 20×20网格 < 500ms
- **指向仿真**: 单次仿真 < 100ms
- **SSE推送**: 延迟 < 1s

## 注意事项

1. **数据质量**: 考古地磁数据存在不确定性，模型结果仅供研究参考
2. **参数校准**: 建议使用实测数据对模型参数进行校准
3. **温度影响**: 环境温度对磁石剩磁有显著影响，需注意温度补偿
4. **安全告警**: 告警阈值可根据实际研究需求调整
5. **数据备份**: 重要研究数据建议定期备份

## 参考文献

1. Constable, C. G., & Johnson, C. L. (2005). CALS7K.2: A continuous geomagnetic field model for the past 7000 years.
2. Stoner, E. C., & Wohlfarth, E. P. (1948). A mechanism of magnetic hysteresis in heterogeneous alloys.
3. 中国古代磁石指向仪器研究 - 自然科学史研究所

## 技术支持

如有问题，请查看各模块源码注释或参考相关技术文档。
