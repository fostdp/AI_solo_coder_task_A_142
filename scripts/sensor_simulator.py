#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
古代司南磁石传感器模拟器
模拟每台司南每1分钟上报磁石磁矩、剩磁强度、指向偏差、环境温度
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
from typing import Dict, List, Optional

import requests
import paho.mqtt.client as mqtt


class SinanSensorSimulator:
    """司南传感器模拟器"""
    
    def __init__(
        self,
        device_id: str,
        device_name: str,
        api_base_url: str = "http://localhost:8080",
        mqtt_broker: str = "localhost",
        mqtt_port: int = 1883,
        mqtt_topic: str = "sinan/sensor",
        location_lat: float = 34.265,
        location_lon: float = 108.955,
        interval: int = 60,
        base_moment: float = 0.025,
        base_remanence: float = 0.85,
        base_temperature: float = 25.0,
        noise_level: float = 0.1,
        use_api: bool = True,
        use_mqtt: bool = False
    ):
        self.device_id = device_id
        self.device_name = device_name
        self.api_base_url = api_base_url
        self.mqtt_broker = mqtt_broker
        self.mqtt_port = mqtt_port
        self.mqtt_topic = mqtt_topic
        self.location_lat = location_lat
        self.location_lon = location_lon
        self.interval = interval
        self.base_moment = base_moment
        self.base_remanence = base_remanence
        self.base_temperature = base_temperature
        self.noise_level = noise_level
        self.use_api = use_api
        self.use_mqtt = use_mqtt
        
        self.running = False
        self.mqtt_client = None
        self.current_azimuth = 0.0
        self.expected_azimuth = 0.0
        self.data_count = 0
        self.alert_count = 0
        
        self._init_mqtt()
    
    def _init_mqtt(self):
        """初始化MQTT客户端"""
        if not self.use_mqtt:
            return
        
        try:
            self.mqtt_client = mqtt.Client(
                client_id=f"simulator_{self.device_id}",
                protocol=mqtt.MQTTv5
            )
            
            def on_connect(client, userdata, flags, rc, properties=None):
                if rc == 0:
                    print(f"[MQTT] 连接成功: {self.mqtt_broker}:{self.mqtt_port}")
                else:
                    print(f"[MQTT] 连接失败，错误码: {rc}")
            
            def on_publish(client, userdata, mid, rc, properties=None):
                if rc == 0:
                    print(f"[MQTT] 消息发布成功，mid={mid}")
                else:
                    print(f"[MQTT] 消息发布失败，错误码: {rc}")
            
            self.mqtt_client.on_connect = on_connect
            self.mqtt_client.on_publish = on_publish
            
            self.mqtt_client.connect_async(
                host=self.mqtt_broker,
                port=self.mqtt_port,
                keepalive=60
            )
            self.mqtt_client.loop_start()
            
        except Exception as e:
            print(f"[MQTT] 初始化失败: {e}")
            self.mqtt_client = None
    
    def _generate_noise(self, base_value: float, noise_scale: float = None) -> float:
        """生成噪声"""
        if noise_scale is None:
            noise_scale = self.noise_level * base_value
        return base_value + random.uniform(-noise_scale, noise_scale)
    
    def _simulate_geomagnetic_field(self, year: int = -100) -> Dict[str, float]:
        """模拟汉代地磁场（简化模型）"""
        intensity = 55000.0
        
        lat_factor = math.cos(math.radians(self.location_lat))
        lon_factor = math.sin(math.radians(self.location_lon))
        
        year_offset = (year + 200) / 1000.0
        intensity_variation = 5000.0 * math.sin(year_offset * math.pi)
        
        return {
            "declination": -2.5 + 0.01 * (self.location_lon - 108.955),
            "inclination": 50.0 + 0.15 * (self.location_lat - 34.265),
            "intensity": intensity + intensity_variation
        }
    
    def _simulate_magnetic_moment(self) -> Dict[str, float]:
        """模拟磁石磁矩"""
        self.current_azimuth += random.uniform(-2.0, 2.0)
        self.current_azimuth = max(-180.0, min(180.0, self.current_azimuth))
        
        if random.random() < 0.02:
            self.current_azimuth += random.choice([-8.0, 8.0])
        
        azimuth_rad = math.radians(self.current_azimuth)
        dip_rad = math.radians(10.0)
        
        magnitude = self._generate_noise(self.base_moment, 0.05 * self.base_moment)
        
        return {
            "x": magnitude * math.cos(azimuth_rad) * math.cos(dip_rad),
            "y": magnitude * math.sin(azimuth_rad) * math.cos(dip_rad),
            "z": magnitude * math.sin(dip_rad),
            "magnitude": magnitude
        }
    
    def _simulate_remanence(self) -> float:
        """模拟剩磁强度"""
        base_remanence = self.base_remanence
        
        time_decay = 0.0001 * self.data_count
        current_remanence = base_remanence - time_decay
        
        return max(0.1, self._generate_noise(current_remanence, 0.02))
    
    def _simulate_temperature(self) -> float:
        """模拟环境温度（带日变化）"""
        hour = datetime.now().hour
        day_cycle = 5.0 * math.sin((hour - 6) * math.pi / 12)
        
        base_temp = self.base_temperature + day_cycle
        return self._generate_noise(base_temp, 0.5)
    
    def _simulate_pointing_deviation(self) -> float:
        """模拟指向偏差"""
        ideal_deviation = abs(self.current_azimuth - self.expected_azimuth)
        
        noise = random.gauss(0, 0.8)
        deviation = abs(ideal_deviation + noise)
        
        if random.random() < 0.05:
            deviation += random.uniform(3.0, 7.0)
        
        return min(deviation, 30.0)
    
    def generate_data(self) -> Dict:
        """生成一条传感器数据"""
        moment = self._simulate_magnetic_moment()
        remanence = self._simulate_remanence()
        temperature = self._simulate_temperature()
        deviation = self._simulate_pointing_deviation()
        
        is_alert = deviation > 5.0
        
        timestamp = datetime.now(timezone.utc).isoformat()
        
        return {
            "device_id": self.device_id,
            "timestamp": timestamp,
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
            "metadata": {
                "simulator": "python",
                "version": "1.0.0",
                "sequence": self.data_count
            }
        }
    
    def send_via_api(self, data: Dict) -> bool:
        """通过HTTP API发送数据"""
        if not self.use_api:
            return True
        
        try:
            url = f"{self.api_base_url}/api/v1/sensor"
            response = requests.post(url, json=data, timeout=10)
            
            if response.status_code == 200 or response.status_code == 201:
                result = response.json()
                print(f"[API] 发送成功: {self.device_id} "
                      f"偏差={data['pointing_deviation']:.2f}° "
                      f"温度={data['environment_temp']:.1f}°C "
                      f"告警={'是' if data['is_alert'] else '否'}")
                return True
            else:
                print(f"[API] 发送失败: HTTP {response.status_code} - {response.text}")
                return False
                
        except requests.RequestException as e:
            print(f"[API] 发送异常: {e}")
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
            
            if result.rc == 0:
                print(f"[MQTT] 发送成功: {topic}")
                return True
            else:
                print(f"[MQTT] 发送失败: rc={result.rc}")
                return False
                
        except Exception as e:
            print(f"[MQTT] 发送异常: {e}")
            return False
    
    def send_data(self, data: Dict) -> bool:
        """发送数据（API和/或MQTT）"""
        api_ok = self.send_via_api(data)
        mqtt_ok = self.send_via_mqtt(data)
        return api_ok and mqtt_ok
    
    def run_once(self) -> Dict:
        """运行一次数据生成和发送"""
        data = self.generate_data()
        self.data_count += 1
        
        if data["is_alert"]:
            self.alert_count += 1
        
        self.send_data(data)
        
        return data
    
    def run(self, duration: Optional[float] = None):
        """持续运行模拟器"""
        self.running = True
        start_time = time.time()
        
        print(f"\n{'='*60}")
        print(f"司南传感器模拟器启动")
        print(f"设备ID: {self.device_id}")
        print(f"设备名称: {self.device_name}")
        print(f"位置: ({self.location_lat}, {self.location_lon})")
        print(f"上报间隔: {self.interval}秒")
        print(f"{'='*60}\n")
        
        def signal_handler(signum, frame):
            print(f"\n\n收到终止信号，准备退出...")
            self.running = False
        
        signal.signal(signal.SIGINT, signal_handler)
        signal.signal(signal.SIGTERM, signal_handler)
        
        try:
            while self.running:
                iteration_start = time.time()
                
                try:
                    self.run_once()
                except Exception as e:
                    print(f"[错误] 运行异常: {e}")
                    import traceback
                    traceback.print_exc()
                
                if duration and (time.time() - start_time) >= duration:
                    print(f"\n达到运行时长 {duration} 秒，准备退出...")
                    break
                
                elapsed = time.time() - iteration_start
                sleep_time = max(0.1, self.interval - elapsed)
                
                for i in range(int(sleep_time)):
                    if not self.running:
                        break
                    time.sleep(1)
                    
                    remaining = int(sleep_time - i)
                    if remaining % 10 == 0 and remaining > 0:
                        sys.stdout.write(f"\r下次上报: {remaining}秒  |  "
                                        f"已发送: {self.data_count}条  |  "
                                        f"告警: {self.alert_count}次")
                        sys.stdout.flush()
                
                sys.stdout.write("\r" + " " * 80 + "\r")
                sys.stdout.flush()
                
        finally:
            self.stop()
    
    def stop(self):
        """停止模拟器"""
        self.running = False
        
        if self.mqtt_client:
            try:
                self.mqtt_client.loop_stop()
                self.mqtt_client.disconnect()
                print("[MQTT] 已断开连接")
            except:
                pass
        
        print(f"\n{'='*60}")
        print(f"模拟器停止")
        print(f"设备ID: {self.device_id}")
        print(f"总共发送: {self.data_count} 条数据")
        print(f"告警次数: {self.alert_count} 次")
        print(f"{'='*60}\n")


class MultiDeviceSimulator:
    """多设备模拟器管理器"""
    
    def __init__(self, devices_config: List[Dict], **kwargs):
        self.simulators = []
        
        for device_cfg in devices_config:
            sim = SinanSensorSimulator(
                device_id=device_cfg["device_id"],
                device_name=device_cfg.get("device_name", device_cfg["device_id"]),
                location_lat=device_cfg.get("lat", 34.265),
                location_lon=device_cfg.get("lon", 108.955),
                interval=device_cfg.get("interval", kwargs.get("interval", 60)),
                **{k: v for k, v in kwargs.items() if k not in ["device_id", "device_name"]}
            )
            self.simulators.append(sim)
    
    def run_all(self, duration: Optional[float] = None):
        """运行所有设备模拟器"""
        import threading
        
        threads = []
        for sim in self.simulators:
            t = threading.Thread(
                target=sim.run,
                args=(duration,),
                daemon=True
            )
            t.start()
            threads.append(t)
            time.sleep(0.5)
        
        try:
            for t in threads:
                t.join()
        except KeyboardInterrupt:
            print("\n收到中断信号，停止所有模拟器...")
            for sim in self.simulators:
                sim.stop()


def get_default_devices() -> List[Dict]:
    """获取默认设备配置"""
    return [
        {
            "device_id": "SINAN-001",
            "device_name": "汉代司南·铜底座原型",
            "lat": 34.265,
            "lon": 108.955,
            "interval": 60
        },
        {
            "device_id": "SINAN-002",
            "device_name": "汉代司南·青铜底座",
            "lat": 34.619,
            "lon": 112.454,
            "interval": 60
        },
        {
            "device_id": "SINAN-003",
            "device_name": "汉代司南·木底座",
            "lat": 28.194,
            "lon": 113.021,
            "interval": 60
        }
    ]


def main():
    parser = argparse.ArgumentParser(
        description="古代司南磁石传感器模拟器",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
示例:
  # 单个设备，使用默认配置
  python sensor_simulator.py --device-id SINAN-001
  
  # 多设备，使用默认设备列表
  python sensor_simulator.py --multi
  
  # 指定API地址和间隔
  python sensor_simulator.py --api-url http://192.168.1.100:8080 --interval 30
  
  # 使用MQTT发送数据
  python sensor_simulator.py --use-mqtt --mqtt-broker localhost
  
  # 运行10分钟后自动退出
  python sensor_simulator.py --duration 600
        """
    )
    
    parser.add_argument(
        "--device-id", "-d",
        type=str,
        default="SINAN-001",
        help="设备ID（单设备模式）"
    )
    
    parser.add_argument(
        "--device-name", "-n",
        type=str,
        default="汉代司南原型",
        help="设备名称"
    )
    
    parser.add_argument(
        "--multi",
        action="store_true",
        help="多设备模式，使用默认设备列表"
    )
    
    parser.add_argument(
        "--api-url",
        type=str,
        default="http://localhost:8080",
        help="后端API地址"
    )
    
    parser.add_argument(
        "--use-api",
        type=lambda x: x.lower() in ['true', '1', 'yes'],
        default=True,
        help="是否使用API发送数据"
    )
    
    parser.add_argument(
        "--use-mqtt",
        action="store_true",
        help="是否使用MQTT发送数据"
    )
    
    parser.add_argument(
        "--mqtt-broker",
        type=str,
        default="localhost",
        help="MQTT broker地址"
    )
    
    parser.add_argument(
        "--mqtt-port",
        type=int,
        default=1883,
        help="MQTT broker端口"
    )
    
    parser.add_argument(
        "--mqtt-topic",
        type=str,
        default="sinan/sensor",
        help="MQTT主题前缀"
    )
    
    parser.add_argument(
        "--lat",
        type=float,
        default=34.265,
        help="纬度"
    )
    
    parser.add_argument(
        "--lon",
        type=float,
        default=108.955,
        help="经度"
    )
    
    parser.add_argument(
        "--interval", "-i",
        type=int,
        default=60,
        help="上报间隔（秒）"
    )
    
    parser.add_argument(
        "--duration",
        type=float,
        default=None,
        help="运行时长（秒），None表示持续运行"
    )
    
    parser.add_argument(
        "--noise",
        type=float,
        default=0.1,
        help="噪声水平"
    )
    
    parser.add_argument(
        "--base-moment",
        type=float,
        default=0.025,
        help="基础磁矩 (A·m²)"
    )
    
    parser.add_argument(
        "--base-remanence",
        type=float,
        default=0.85,
        help="基础剩磁强度"
    )
    
    parser.add_argument(
        "--base-temp",
        type=float,
        default=25.0,
        help="基础环境温度 (°C)"
    )
    
    parser.add_argument(
        "--config-file", "-c",
        type=str,
        default=None,
        help="从JSON配置文件加载设备配置"
    )
    
    args = parser.parse_args()
    
    if args.config_file:
        try:
            with open(args.config_file, 'r', encoding='utf-8') as f:
                config = json.load(f)
                devices = config.get("devices", get_default_devices())
                
                common_cfg = config.get("common", {})
                for k, v in common_cfg.items():
                    if not hasattr(args, k) or getattr(args, k) == parser.get_default(k):
                        setattr(args, k, v)
                        
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
            "lat": args.lat,
            "lon": args.lon,
            "interval": args.interval
        }]
    
    print(f"\n{'='*60}")
    print(f"古代司南磁石传感器模拟器")
    print(f"{'='*60}")
    print(f"设备数量: {len(devices)}")
    for dev in devices:
        print(f"  - {dev['device_id']}: {dev['device_name']} "
              f"@({dev['lat']:.3f}, {dev['lon']:.3f})")
    print(f"API: {args.api_url if args.use_api else '禁用'}")
    print(f"MQTT: {args.mqtt_broker}:{args.mqtt_port if args.use_mqtt else '禁用'}")
    print(f"上报间隔: {args.interval}秒")
    if args.duration:
        print(f"运行时长: {args.duration}秒")
    print(f"{'='*60}")
    
    if len(devices) > 1:
        simulator = MultiDeviceSimulator(
            devices,
            api_base_url=args.api_url,
            mqtt_broker=args.mqtt_broker,
            mqtt_port=args.mqtt_port,
            mqtt_topic=args.mqtt_topic,
            noise_level=args.noise,
            base_moment=args.base_moment,
            base_remanence=args.base_remanence,
            base_temperature=args.base_temp,
            use_api=args.use_api,
            use_mqtt=args.use_mqtt
        )
        simulator.run_all(args.duration)
    else:
        dev = devices[0]
        simulator = SinanSensorSimulator(
            device_id=dev["device_id"],
            device_name=dev["device_name"],
            api_base_url=args.api_url,
            mqtt_broker=args.mqtt_broker,
            mqtt_port=args.mqtt_port,
            mqtt_topic=args.mqtt_topic,
            location_lat=dev["lat"],
            location_lon=dev["lon"],
            interval=dev["interval"],
            noise_level=args.noise,
            base_moment=args.base_moment,
            base_remanence=args.base_remanence,
            base_temperature=args.base_temp,
            use_api=args.use_api,
            use_mqtt=args.use_mqtt
        )
        simulator.run(args.duration)


if __name__ == "__main__":
    main()
