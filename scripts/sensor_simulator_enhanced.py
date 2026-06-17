#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
古代司南磁石传感器模拟器 (增强版)
支持多种磁石材料配置、地磁场条件设置、故障模拟
"""

import argparse
import json
import math
import os
import random
import signal
import sys
import time
from datetime import datetime, timezone
from typing import Dict, List, Optional, Tuple

import requests
import paho.mqtt.client as mqtt


# ====================
# 磁石材料配置
# ====================
MAGNET_MATERIALS: Dict[str, Dict] = {
    "magnetite_natural": {
        "name": "天然磁铁矿 (Fe₃O₄)",
        "description": "汉代司南使用的天然磁石，剩磁较低，磁矩较小",
        "remanence_range": (0.03, 0.08),
        "moment_range": (0.01, 0.03),
        "coercivity": 15.0,
        "temperature_coefficient": -0.002,
        "density": 5.18,
        "color": "#8B4513"
    },
    "magnetite_pure": {
        "name": "高纯磁铁矿",
        "description": "精选天然磁铁矿，纯度高，剩磁稳定",
        "remanence_range": (0.06, 0.12),
        "moment_range": (0.02, 0.05),
        "coercivity": 20.0,
        "temperature_coefficient": -0.0015,
        "density": 5.18,
        "color": "#4A4A4A"
    },
    "alnico": {
        "name": "铝镍钴合金",
        "description": "现代磁钢，高剩磁，温度稳定性好",
        "remanence_range": (0.8, 1.35),
        "moment_range": (0.1, 0.3),
        "coercivity": 50.0,
        "temperature_coefficient": -0.0002,
        "density": 7.3,
        "color": "#C0C0C0"
    },
    "neodymium": {
        "name": "钕铁硼 (NdFeB)",
        "description": "现代稀土永磁，极强磁性能",
        "remanence_range": (1.0, 1.4),
        "moment_range": (0.3, 0.8),
        "coercivity": 1000.0,
        "temperature_coefficient": -0.0011,
        "density": 7.5,
        "color": "#4682B4"
    },
    "ferrite": {
        "name": "铁氧体",
        "description": "陶瓷永磁材料，成本低",
        "remanence_range": (0.2, 0.4),
        "moment_range": (0.05, 0.15),
        "coercivity": 150.0,
        "temperature_coefficient": -0.002,
        "density": 4.9,
        "color": "#2F4F4F"
    },
    "smco": {
        "name": "钐钴合金",
        "description": "稀土永磁，高温稳定性极佳",
        "remanence_range": (0.8, 1.1),
        "moment_range": (0.2, 0.5),
        "coercivity": 700.0,
        "temperature_coefficient": -0.0003,
        "density": 8.4,
        "color": "#708090"
    }
}

# ====================
# 地磁场历史时期配置
# ====================
GEOMAGNETIC_PERIODS: Dict[str, Dict] = {
    "modern": {
        "name": "现代 (2000年)",
        "description": "当前地磁场强度",
        "intensity_factor": 1.0,
        "declination_offset": 0.0,
        "inclination_offset": 0.0,
        "year": 2000.0
    },
    "han_dynasty": {
        "name": "汉代 (公元前100年)",
        "description": "汉代地磁场强度（约比现代强15%）",
        "intensity_factor": 1.15,
        "declination_offset": -2.5,
        "inclination_offset": 1.0,
        "year": -100.0
    },
    "tang_dynasty": {
        "name": "唐代 (公元700年)",
        "description": "唐代地磁场强度",
        "intensity_factor": 1.08,
        "declination_offset": -1.0,
        "inclination_offset": 0.5,
        "year": 700.0
    },
    "song_dynasty": {
        "name": "宋代 (公元1100年)",
        "description": "宋代地磁场强度（地磁偏角首次记录）",
        "intensity_factor": 1.03,
        "declination_offset": -5.0,
        "inclination_offset": 0.0,
        "year": 1100.0
    },
    "laschamp_event": {
        "name": "Laschamp 地磁漂移 (4万年前)",
        "description": "地磁反转时期，磁场强度骤降",
        "intensity_factor": 0.25,
        "declination_offset": 180.0,
        "inclination_offset": -20.0,
        "year": -40000.0
    },
    "low_intensity": {
        "name": "低磁场环境",
        "description": "磁场强度仅为现代的30%",
        "intensity_factor": 0.3,
        "declination_offset": 0.0,
        "inclination_offset": 0.0,
        "year": 0.0
    },
    "high_intensity": {
        "name": "强磁场环境",
        "description": "磁场强度为现代的200%",
        "intensity_factor": 2.0,
        "declination_offset": 0.0,
        "inclination_offset": 0.0,
        "year": 0.0
    }
}

# ====================
# 故障模式配置
# ====================
FAILURE_MODES: Dict[str, Dict] = {
    "none": {
        "name": "正常运行",
        "description": "无故障，正常工作"
    },
    "noise_high": {
        "name": "高噪声",
        "description": "传感器噪声放大3倍",
        "noise_multiplier": 3.0
    },
    "demagnetization": {
        "name": "磁石退磁",
        "description": "磁矩逐渐衰减，剩磁降低",
        "moment_decay_rate": 0.0001,
        "remanence_decay_rate": 0.00005
    },
    "sensor_drift": {
        "name": "传感器漂移",
        "description": "读数持续单向漂移",
        "azimuth_drift_rate": 0.01
    },
    "interference_magnetic": {
        "name": "强磁干扰",
        "description": "存在额外磁场干扰源",
        "interference_field": 10000.0,
        "interference_direction": (1, 0, 0)
    },
    "temperature_extreme": {
        "name": "极端温度",
        "description": "超出正常工作温度范围",
        "temp_range": (-30, 80)
    },
    "spikes": {
        "name": "数据尖峰",
        "description": "偶发极端异常数据",
        "spike_probability": 0.05,
        "spike_multiplier": 10.0
    }
}


class EnhancedSinanSimulator:
    """增强版司南传感器模拟器"""

    def __init__(
        self,
        device_id: str,
        device_name: str,
        material: str = "magnetite_natural",
        geomagnetic_period: str = "han_dynasty",
        failure_mode: str = "none",
        api_base_url: str = "http://localhost:8080",
        mqtt_broker: str = "localhost",
        mqtt_port: int = 1883,
        mqtt_topic: str = "sinan/sensor",
        mqtt_username: str = "",
        mqtt_password: str = "",
        location_lat: float = 34.265,
        location_lon: float = 108.955,
        interval_ms: int = 1000,
        noise_level: float = 0.1,
        use_api: bool = True,
        use_mqtt: bool = False,
        alert_threshold: float = 5.0,
        custom_moment: Optional[float] = None,
        custom_remanence: Optional[float] = None
    ):
        self.device_id = device_id
        self.device_name = device_name
        self.api_base_url = api_base_url
        self.mqtt_broker = mqtt_broker
        self.mqtt_port = mqtt_port
        self.mqtt_topic = mqtt_topic
        self.mqtt_username = mqtt_username
        self.mqtt_password = mqtt_password
        self.location_lat = location_lat
        self.location_lon = location_lon
        self.interval_ms = interval_ms
        self.noise_level = noise_level
        self.use_api = use_api
        self.use_mqtt = use_mqtt
        self.alert_threshold = alert_threshold

        # 加载磁石材料配置
        if material not in MAGNET_MATERIALS:
            raise ValueError(f"未知磁石材料: {material}. 可用: {list(MAGNET_MATERIALS.keys())}")
        self.material = material
        self.material_config = MAGNET_MATERIALS[material]

        # 加载地磁场时期配置
        if geomagnetic_period not in GEOMAGNETIC_PERIODS:
            raise ValueError(f"未知地磁场时期: {geomagnetic_period}. 可用: {list(GEOMAGNETIC_PERIODS.keys())}")
        self.geomagnetic_period = geomagnetic_period
        self.geo_config = GEOMAGNETIC_PERIODS[geomagnetic_period]

        # 加载故障模式配置
        if failure_mode not in FAILURE_MODES:
            raise ValueError(f"未知故障模式: {failure_mode}. 可用: {list(FAILURE_MODES.keys())}")
        self.failure_mode = failure_mode
        self.failure_config = FAILURE_MODES[failure_mode]

        # 自定义磁矩和剩磁
        rem_range = self.material_config["remanence_range"]
        mom_range = self.material_config["moment_range"]
        self.base_remanence = custom_remanence if custom_remanence else random.uniform(*rem_range)
        self.base_moment = custom_moment if custom_moment else random.uniform(*mom_range)

        # 运行状态
        self.running = False
        self.mqtt_client = None
        self.data_count = 0
        self.alert_count = 0
        self.current_azimuth = 0.0
        self.expected_azimuth = 0.0
        self.cumulative_drift = 0.0
        self.current_moment = self.base_moment
        self.current_remanence = self.base_remanence

        self._init_mqtt()

    def _init_mqtt(self):
        """初始化MQTT客户端"""
        if not self.use_mqtt:
            return

        try:
            self.mqtt_client = mqtt.Client(
                client_id=f"sim_{self.device_id}",
                protocol=mqtt.MQTTv5
            )

            if self.mqtt_username:
                self.mqtt_client.username_pw_set(
                    self.mqtt_username,
                    self.mqtt_password
                )

            def on_connect(client, userdata, flags, rc, properties=None):
                if rc == 0:
                    print(f"[MQTT] {self.device_id} 连接成功: {self.mqtt_broker}:{self.mqtt_port}")
                else:
                    print(f"[MQTT] {self.device_id} 连接失败，错误码: {rc}")

            self.mqtt_client.on_connect = on_connect

            self.mqtt_client.connect_async(
                host=self.mqtt_broker,
                port=self.mqtt_port,
                keepalive=60
            )
            self.mqtt_client.loop_start()

        except Exception as e:
            print(f"[MQTT] {self.device_id} 初始化失败: {e}")
            self.mqtt_client = None

    def _get_effective_noise(self) -> float:
        """获取有效噪声水平（考虑故障模式）"""
        noise = self.noise_level
        if self.failure_mode == "noise_high":
            noise *= self.failure_config.get("noise_multiplier", 1.0)
        return noise

    def _apply_demagnetization(self):
        """应用退磁效应"""
        if self.failure_mode == "demagnetization":
            decay_rate = self.failure_config.get("moment_decay_rate", 0)
            rem_decay = self.failure_config.get("remanence_decay_rate", 0)
            self.current_moment *= (1 - decay_rate)
            self.current_remanence *= (1 - rem_decay)

    def _apply_drift(self):
        """应用传感器漂移"""
        if self.failure_mode == "sensor_drift":
            drift_rate = self.failure_config.get("azimuth_drift_rate", 0)
            self.cumulative_drift += drift_rate

    def _get_geomagnetic_field(self) -> Dict[str, float]:
        """计算当前位置的地磁场（考虑时期配置）"""
        base_intensity = 55000.0

        lat_factor = math.cos(math.radians(self.location_lat))
        lon_factor = math.sin(math.radians(self.location_lon))

        intensity = base_intensity * self.geo_config["intensity_factor"]
        declination = -2.5 + 0.01 * (self.location_lon - 108.955) + self.geo_config["declination_offset"]
        inclination = 50.0 + 0.15 * (self.location_lat - 34.265) + self.geo_config["inclination_offset"]

        if self.failure_mode == "interference_magnetic":
            intensity += self.failure_config.get("interference_field", 0)

        return {
            "declination": declination,
            "inclination": inclination,
            "intensity": intensity,
            "period": self.geomagnetic_period
        }

    def _simulate_magnetic_moment(self, geo_field: Dict[str, float]) -> Dict[str, float]:
        """模拟磁矩（考虑材料属性和地磁场）"""
        noise = self._get_effective_noise()

        self.current_azimuth += random.uniform(-1.0, 1.0) * (1 + noise)
        self.current_azimuth = max(-180.0, min(180.0, self.current_azimuth))

        self._apply_drift()
        effective_azimuth = self.current_azimuth + self.cumulative_drift

        if random.random() < 0.01:
            effective_azimuth += random.choice([-5.0, 5.0])

        if self.failure_mode == "spikes" and random.random() < self.failure_config.get("spike_probability", 0):
            effective_azimuth *= self.failure_config.get("spike_multiplier", 1.0)

        azimuth_rad = math.radians(effective_azimuth)
        dip_rad = math.radians(geo_field["inclination"] * 0.2)

        self._apply_demagnetization()
        magnitude = self.current_moment * (1 + random.uniform(-noise * 0.5, noise * 0.5))

        temp_correction = 1.0
        if self.material_config.get("temperature_coefficient"):
            temp = self._current_temperature()
            temp_correction = 1.0 + self.material_config["temperature_coefficient"] * (temp - 25.0)

        magnitude *= temp_correction

        return {
            "x": magnitude * math.cos(azimuth_rad) * math.cos(dip_rad),
            "y": magnitude * math.sin(azimuth_rad) * math.cos(dip_rad),
            "z": magnitude * math.sin(dip_rad),
            "magnitude": magnitude
        }

    def _current_temperature(self) -> float:
        """获取当前环境温度"""
        hour = datetime.now().hour
        day_cycle = 5.0 * math.sin((hour - 6) * math.pi / 12)

        if self.failure_mode == "temperature_extreme":
            t_range = self.failure_config.get("temp_range", (-30, 80))
            base_temp = random.uniform(*t_range)
        else:
            base_temp = 25.0 + day_cycle

        noise = self._get_effective_noise() * 5
        return base_temp + random.uniform(-noise, noise)

    def _simulate_remanence(self, temperature: float) -> float:
        """模拟剩磁强度"""
        remanence = self.current_remanence

        temp_factor = 1.0
        if self.material_config.get("temperature_coefficient"):
            temp_factor = 1.0 + self.material_config["temperature_coefficient"] * (temperature - 25.0)

        noise = self._get_effective_noise() * 0.02
        return max(0.01, remanence * temp_factor * (1 + random.uniform(-noise, noise)))

    def _simulate_pointing_deviation(self, geo_field: Dict[str, float]) -> float:
        """模拟指向偏差"""
        field_ratio = geo_field["intensity"] / 55000.0

        ideal_deviation = abs(self.current_azimuth + self.cumulative_drift - self.expected_azimuth)

        noise_std = 0.8 / max(field_ratio, 0.3)
        noise = random.gauss(0, noise_std)

        deviation = abs(ideal_deviation + noise)

        if random.random() < 0.03:
            deviation += random.uniform(2.0, 6.0)

        if field_ratio < 0.5:
            deviation += (0.5 - field_ratio) * 20

        return min(deviation, 45.0)

    def generate_data(self) -> Dict:
        """生成一条传感器数据"""
        geo_field = self._get_geomagnetic_field()
        moment = self._simulate_magnetic_moment(geo_field)
        temperature = self._current_temperature()
        remanence = self._simulate_remanence(temperature)
        deviation = self._simulate_pointing_deviation(geo_field)

        is_alert = deviation > self.alert_threshold

        timestamp = datetime.now(timezone.utc).isoformat()

        return {
            "device_id": self.device_id,
            "device_name": self.device_name,
            "timestamp": timestamp,
            "sequence": self.data_count,
            "magnetic_moment_x": moment["x"],
            "magnetic_moment_y": moment["y"],
            "magnetic_moment_z": moment["z"],
            "magnetic_moment_magnitude": moment["magnitude"],
            "remanence": remanence,
            "pointing_deviation": deviation,
            "environment_temp": temperature,
            "location_lat": self.location_lat,
            "location_lon": self.location_lon,
            "is_alert": is_alert,
            "geomagnetic_field": geo_field,
            "material": {
                "type": self.material,
                "name": self.material_config["name"],
                "base_moment": self.base_moment,
                "base_remanence": self.base_remanence,
                "current_moment": self.current_moment,
                "current_remanence": self.current_remanence
            },
            "failure_mode": self.failure_mode,
            "simulation": {
                "version": "2.0.0",
                "geomagnetic_period": self.geomagnetic_period
            }
        }

    def send_via_api(self, data: Dict) -> bool:
        """通过HTTP API发送数据"""
        if not self.use_api:
            return True

        try:
            url = f"{self.api_base_url}/api/v1/sensor"
            response = requests.post(url, json=data, timeout=5)

            if response.status_code in (200, 201):
                print(f"[API] {self.device_id} | 偏差={data['pointing_deviation']:5.2f}° | "
                      f"温度={data['environment_temp']:5.1f}°C | 剩磁={data['remanence']:.4f}T | "
                      f"告警={'⚠️ ' if data['is_alert'] else '✓ '}")
                return True
            else:
                print(f"[API] {self.device_id} 发送失败: HTTP {response.status_code}")
                return False

        except requests.RequestException as e:
            print(f"[API] {self.device_id} 异常: {str(e)[:50]}")
            return False

    def send_via_mqtt(self, data: Dict) -> bool:
        """通过MQTT发送数据"""
        if not self.use_mqtt or not self.mqtt_client:
            return True

        try:
            topic = f"{self.mqtt_topic}/{self.device_id}"
            payload = json.dumps(data)

            result = self.mqtt_client.publish(
                topic=topic,
                payload=payload,
                qos=1
            )

            return result.rc == 0

        except Exception as e:
            print(f"[MQTT] {self.device_id} 异常: {e}")
            return False

    def run_once(self) -> Dict:
        """运行一次数据生成和发送"""
        data = self.generate_data()
        self.data_count += 1

        if data["is_alert"]:
            self.alert_count += 1

        self.send_via_api(data)
        self.send_via_mqtt(data)

        return data

    def run(self, duration: Optional[float] = None):
        """持续运行模拟器"""
        self.running = True
        start_time = time.time()

        self._print_header()

        def signal_handler(signum, frame):
            print(f"\n\n[{self.device_id}] 收到终止信号，准备退出...")
            self.running = False

        signal.signal(signal.SIGINT, signal_handler)
        signal.signal(signal.SIGTERM, signal_handler)

        try:
            while self.running:
                iteration_start = time.time()

                try:
                    self.run_once()
                except Exception as e:
                    print(f"[{self.device_id}] 运行异常: {e}")
                    import traceback
                    traceback.print_exc()

                if duration and (time.time() - start_time) >= duration:
                    print(f"\n[{self.device_id}] 达到运行时长 {duration} 秒")
                    break

                elapsed = (time.time() - iteration_start) * 1000
                sleep_time = max(1, self.interval_ms - elapsed)
                time.sleep(sleep_time / 1000)

        finally:
            self.stop()

    def _print_header(self):
        """打印模拟器启动信息"""
        print(f"\n{'='*80}")
        print(f"🎯 司南磁石传感器模拟器 (增强版 v2.0)")
        print(f"{'='*80}")
        print(f"设备 ID:    {self.device_id}")
        print(f"设备名称:   {self.device_name}")
        print(f"位置:       ({self.location_lat:.4f}, {self.location_lon:.4f})")
        print(f"磁石材料:   {self.material_config['name']}")
        print(f"            {self.material_config['description']}")
        print(f"基础磁矩:   {self.base_moment:.4f} A·m²")
        print(f"基础剩磁:   {self.base_remanence:.4f} T")
        print(f"地磁场期:   {self.geo_config['name']}")
        print(f"            {self.geo_config['description']}")
        print(f"场强因子:   x{self.geo_config['intensity_factor']}")
        print(f"故障模式:   {self.failure_config['name']}")
        print(f"            {self.failure_config['description']}")
        print(f"上报间隔:   {self.interval_ms}ms")
        print(f"告警阈值:   {self.alert_threshold}°")
        print(f"API:        {self.api_base_url if self.use_api else '禁用'}")
        print(f"MQTT:       {self.mqtt_broker}:{self.mqtt_port if self.use_mqtt else '禁用'}")
        print(f"{'='*80}\n")

    def stop(self):
        """停止模拟器"""
        self.running = False

        if self.mqtt_client:
            try:
                self.mqtt_client.loop_stop()
                self.mqtt_client.disconnect()
            except:
                pass

        print(f"\n{'='*60}")
        print(f"模拟器停止: {self.device_id}")
        print(f"总共发送: {self.data_count} 条数据")
        print(f"告警次数: {self.alert_count} 次")
        print(f"当前磁矩: {self.current_moment:.6f} A·m² (原始 {self.base_moment:.6f})")
        print(f"当前剩磁: {self.current_remanence:.6f} T (原始 {self.base_remanence:.6f})")
        print(f"{'='*60}\n")


def list_materials():
    """列出所有可用磁石材料"""
    print(f"\n{'='*80}")
    print(f"📋 可用磁石材料列表")
    print(f"{'='*80}")
    for key, cfg in MAGNET_MATERIALS.items():
        print(f"\n🔹 {key}")
        print(f"   名称: {cfg['name']}")
        print(f"   描述: {cfg['description']}")
        print(f"   剩磁范围: {cfg['remanence_range'][0]:.3f} - {cfg['remanence_range'][1]:.3f} T")
        print(f"   磁矩范围: {cfg['moment_range'][0]:.4f} - {cfg['moment_range'][1]:.4f} A·m²")
        print(f"   矫顽力: {cfg['coercivity']} kA/m")
        print(f"   温度系数: {cfg['temperature_coefficient']} /°C")
        print(f"   密度: {cfg['density']} g/cm³")
    print(f"\n{'='*80}\n")


def list_periods():
    """列出所有可用地磁场时期"""
    print(f"\n{'='*80}")
    print(f"📋 可用地磁场时期列表")
    print(f"{'='*80}")
    for key, cfg in GEOMAGNETIC_PERIODS.items():
        print(f"\n🔹 {key}")
        print(f"   名称: {cfg['name']}")
        print(f"   描述: {cfg['description']}")
        print(f"   场强因子: x{cfg['intensity_factor']}")
        print(f"   偏角偏移: {cfg['declination_offset']:+.1f}°")
        print(f"   倾角偏移: {cfg['inclination_offset']:+.1f}°")
        if cfg['year'] != 0:
            print(f"   对应年份: {cfg['year']:+.0f} 年")
    print(f"\n{'='*80}\n")


def list_failures():
    """列出所有可用故障模式"""
    print(f"\n{'='*80}")
    print(f"📋 可用故障模式列表")
    print(f"{'='*80}")
    for key, cfg in FAILURE_MODES.items():
        print(f"\n🔹 {key}")
        print(f"   名称: {cfg['name']}")
        print(f"   描述: {cfg['description']}")
    print(f"\n{'='*80}\n")


def get_default_devices() -> List[Dict]:
    """获取默认多设备配置（不同材料对比测试）"""
    return [
        {
            "device_id": "SINAN-HAN-001",
            "device_name": "汉代司南·天然磁铁矿",
            "material": "magnetite_natural",
            "geomagnetic_period": "han_dynasty",
            "lat": 34.265,
            "lon": 108.955,
            "interval_ms": 1000
        },
        {
            "device_id": "SINAN-MOD-001",
            "device_name": "现代司南·钕铁硼",
            "material": "neodymium",
            "geomagnetic_period": "modern",
            "lat": 34.265,
            "lon": 108.955,
            "interval_ms": 1000
        },
        {
            "device_id": "SINAN-WEAK-001",
            "device_name": "低磁场测试·铁氧体",
            "material": "ferrite",
            "geomagnetic_period": "low_intensity",
            "lat": 34.265,
            "lon": 108.955,
            "interval_ms": 1000
        },
        {
            "device_id": "SINAN-FAIL-001",
            "device_name": "故障模拟·退磁",
            "material": "magnetite_pure",
            "geomagnetic_period": "modern",
            "failure_mode": "demagnetization",
            "lat": 34.265,
            "lon": 108.955,
            "interval_ms": 500
        }
    ]


def run_multi_simulator(devices: List[Dict], common_config: Dict):
    """运行多设备模拟器"""
    import threading

    threads = []
    simulators = []

    for dev_cfg in devices:
        sim = EnhancedSinanSimulator(
            device_id=dev_cfg["device_id"],
            device_name=dev_cfg.get("device_name", dev_cfg["device_id"]),
            material=dev_cfg.get("material", "magnetite_natural"),
            geomagnetic_period=dev_cfg.get("geomagnetic_period", "han_dynasty"),
            failure_mode=dev_cfg.get("failure_mode", "none"),
            location_lat=dev_cfg.get("lat", 34.265),
            location_lon=dev_cfg.get("lon", 108.955),
            interval_ms=dev_cfg.get("interval_ms", common_config.get("interval_ms", 1000)),
            api_base_url=common_config.get("api_base_url", "http://localhost:8080"),
            mqtt_broker=common_config.get("mqtt_broker", "localhost"),
            mqtt_port=common_config.get("mqtt_port", 1883),
            mqtt_topic=common_config.get("mqtt_topic", "sinan/sensor"),
            mqtt_username=common_config.get("mqtt_username", ""),
            mqtt_password=common_config.get("mqtt_password", ""),
            noise_level=common_config.get("noise_level", 0.1),
            use_api=common_config.get("use_api", True),
            use_mqtt=common_config.get("use_mqtt", False),
            alert_threshold=common_config.get("alert_threshold", 5.0),
            custom_moment=dev_cfg.get("custom_moment"),
            custom_remanence=dev_cfg.get("custom_remanence")
        )
        simulators.append(sim)

        t = threading.Thread(
            target=sim.run,
            args=(common_config.get("duration"),),
            daemon=True
        )
        t.start()
        threads.append(t)
        time.sleep(0.3)

    try:
        for t in threads:
            t.join()
    except KeyboardInterrupt:
        print("\n收到中断信号，停止所有模拟器...")
        for sim in simulators:
            sim.stop()


def main():
    import sys
    if sys.platform.startswith('win'):
        sys.stdout.reconfigure(encoding='utf-8')
        sys.stderr.reconfigure(encoding='utf-8')

    parser = argparse.ArgumentParser(
        description="古代司南磁石传感器模拟器 (增强版 v2.0)",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
====================
📚 示例用法
====================

1️⃣  基础用法 - 汉代司南（天然磁铁矿）:
   python sensor_simulator_enhanced.py \\
       --device-id SINAN-001 \\
       --device-name "汉代司南原型" \\
       --material magnetite_natural \\
       --period han_dynasty \\
       --interval 1000

2️⃣  现代磁钢对比测试:
   python sensor_simulator_enhanced.py \\
       --device-id MODERN-001 \\
       --material neodymium \\
       --period modern \\
       --custom-moment 0.5 \\
       --custom-remanence 1.2

3️⃣  故障模拟 - 退磁效应:
   python sensor_simulator_enhanced.py \\
       --device-id DEMAG-001 \\
       --material magnetite_pure \\
       --failure demagnetization \\
       --interval 500

4️⃣  低磁场环境测试:
   python sensor_simulator_enhanced.py \\
       --device-id LOWFIELD-001 \\
       --material ferrite \\
       --period low_intensity \\
       --alert-threshold 8.0

5️⃣  多设备对比（不同材料同时上报）:
   python sensor_simulator_enhanced.py --multi

6️⃣  列出所有可用配置:
   python sensor_simulator_enhanced.py --list-materials
   python sensor_simulator_enhanced.py --list-periods
   python sensor_simulator_enhanced.py --list-failures

7️⃣  使用MQTT上报 + API上报:
   python sensor_simulator_enhanced.py \\
       --use-mqtt --mqtt-broker mosquitto \\
       --mqtt-topic sinan/sensor/data
        """
    )

    parser.add_argument("--device-id", "-d", type=str, default="SINAN-SIM-001", help="设备ID")
    parser.add_argument("--device-name", "-n", type=str, default="司南模拟器", help="设备名称")

    parser.add_argument("--material", "-m", type=str, default="magnetite_natural",
                        help=f"磁石材料类型。可用: {list(MAGNET_MATERIALS.keys())}")
    parser.add_argument("--period", "-p", type=str, default="han_dynasty",
                        help=f"地磁场时期。可用: {list(GEOMAGNETIC_PERIODS.keys())}")
    parser.add_argument("--failure", "-f", type=str, default="none",
                        help=f"故障模式。可用: {list(FAILURE_MODES.keys())}")

    parser.add_argument("--custom-moment", type=float, default=None, help="自定义磁矩 (A·m²)")
    parser.add_argument("--custom-remanence", type=float, default=None, help="自定义剩磁 (T)")

    parser.add_argument("--lat", type=float, default=34.265, help="纬度")
    parser.add_argument("--lon", type=float, default=108.955, help="经度")

    parser.add_argument("--interval", "-i", type=int, default=1000, help="上报间隔 (毫秒)")
    parser.add_argument("--duration", type=float, default=None, help="运行时长 (秒)")
    parser.add_argument("--noise", type=float, default=0.1, help="噪声水平")
    parser.add_argument("--alert-threshold", type=float, default=5.0, help="告警阈值 (度)")

    parser.add_argument("--api-url", type=str, default="http://localhost:8080", help="后端API地址")
    parser.add_argument("--use-api", type=lambda x: x.lower() in ['true', '1', 'yes'], default=True, help="是否使用API")

    parser.add_argument("--use-mqtt", action="store_true", help="是否使用MQTT上报")
    parser.add_argument("--mqtt-broker", type=str, default="localhost", help="MQTT broker地址")
    parser.add_argument("--mqtt-port", type=int, default=1883, help="MQTT broker端口")
    parser.add_argument("--mqtt-topic", type=str, default="sinan/sensor", help="MQTT主题前缀")
    parser.add_argument("--mqtt-username", type=str, default="", help="MQTT用户名")
    parser.add_argument("--mqtt-password", type=str, default="", help="MQTT密码")

    parser.add_argument("--multi", action="store_true", help="多设备模式（默认4设备对比）")
    parser.add_argument("--config-file", "-c", type=str, default=None, help="JSON配置文件路径")

    parser.add_argument("--list-materials", action="store_true", help="列出所有磁石材料")
    parser.add_argument("--list-periods", action="store_true", help="列出所有地磁场时期")
    parser.add_argument("--list-failures", action="store_true", help="列出所有故障模式")

    args = parser.parse_args()

    if args.list_materials:
        list_materials()
        return
    if args.list_periods:
        list_periods()
        return
    if args.list_failures:
        list_failures()
        return

    common_config = {
        "api_base_url": os.environ.get("SIMULATOR_BACKEND_URL", args.api_url),
        "mqtt_broker": os.environ.get("SIMULATOR_MQTT_URL", args.mqtt_broker).replace("mqtt://", ""),
        "mqtt_port": args.mqtt_port,
        "mqtt_topic": args.mqtt_topic,
        "mqtt_username": args.mqtt_username,
        "mqtt_password": args.mqtt_password,
        "noise_level": args.noise,
        "use_api": args.use_api,
        "use_mqtt": args.use_mqtt,
        "alert_threshold": args.alert_threshold,
        "duration": args.duration,
        "interval_ms": args.interval
    }

    if args.config_file:
        try:
            with open(args.config_file, 'r', encoding='utf-8') as f:
                config = json.load(f)
                devices = config.get("devices", get_default_devices())
                file_common = config.get("common", {})
                for k, v in file_common.items():
                    if k not in common_config or common_config[k] == parser.get_default(k):
                        common_config[k] = v
            print(f"[配置] 从文件加载: {args.config_file}")
        except Exception as e:
            print(f"[错误] 无法加载配置文件: {e}")
            sys.exit(1)
    elif args.multi:
        devices = get_default_devices()
    else:
        devices = [{
            "device_id": args.device_id,
            "device_name": args.device_name,
            "material": args.material,
            "geomagnetic_period": args.period,
            "failure_mode": args.failure,
            "lat": args.lat,
            "lon": args.lon,
            "interval_ms": args.interval,
            "custom_moment": args.custom_moment,
            "custom_remanence": args.custom_remanence
        }]

    if len(devices) > 1:
        run_multi_simulator(devices, common_config)
    else:
        dev = devices[0]
        sim = EnhancedSinanSimulator(
            device_id=dev["device_id"],
            device_name=dev.get("device_name", dev["device_id"]),
            material=dev.get("material", "magnetite_natural"),
            geomagnetic_period=dev.get("geomagnetic_period", "han_dynasty"),
            failure_mode=dev.get("failure_mode", "none"),
            location_lat=dev.get("lat", 34.265),
            location_lon=dev.get("lon", 108.955),
            interval_ms=dev.get("interval_ms", common_config["interval_ms"]),
            api_base_url=common_config["api_base_url"],
            mqtt_broker=common_config["mqtt_broker"],
            mqtt_port=common_config["mqtt_port"],
            mqtt_topic=common_config["mqtt_topic"],
            mqtt_username=common_config["mqtt_username"],
            mqtt_password=common_config["mqtt_password"],
            noise_level=common_config["noise_level"],
            use_api=common_config["use_api"],
            use_mqtt=common_config["use_mqtt"],
            alert_threshold=common_config["alert_threshold"],
            custom_moment=dev.get("custom_moment"),
            custom_remanence=dev.get("custom_remanence")
        )
        sim.run(common_config["duration"])


if __name__ == "__main__":
    main()
