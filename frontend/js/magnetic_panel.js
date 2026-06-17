class MagneticPanel {
    constructor() {
        this.vectorFieldRenderer = null;
        this.chartManager = null;
        this.currentGeomagneticField = null;
        this.selectedDeviceId = null;
        this.dataCache = {
            deviation: [],
            simulationResults: []
        };
    }

    init() {
        this.vectorFieldRenderer = new VectorFieldRenderer('vectorFieldCanvas');
        this.chartManager = new ChartManager();
        this.chartManager.init();
        this.bindEvents();
        this.initDefaults();
    }

    bindEvents() {
        document.getElementById('calcFieldBtn').addEventListener('click', () => this.calculateField());
        document.getElementById('genVectorBtn').addEventListener('click', () => this.generateVectorField());
        document.getElementById('runSimulationBtn').addEventListener('click', () => this.runSimulation());
        document.getElementById('refreshSimBtn').addEventListener('click', () => this.loadSimulationResults());

        const slider = document.getElementById('gridSize');
        const valueSpan = document.getElementById('gridSizeValue');
        slider.addEventListener('input', () => {
            valueSpan.textContent = slider.value;
        });

        document.getElementById('showArrows').addEventListener('change', (e) => {
            if (this.vectorFieldRenderer) {
                this.vectorFieldRenderer.showArrows = e.target.checked;
                this.vectorFieldRenderer.render();
            }
        });
        document.getElementById('showHeatmap').addEventListener('change', (e) => {
            if (this.vectorFieldRenderer) {
                this.vectorFieldRenderer.showHeatmap = e.target.checked;
                this.vectorFieldRenderer.render();
            }
        });
        document.getElementById('animateField').addEventListener('change', (e) => {
            if (this.vectorFieldRenderer) {
                this.vectorFieldRenderer.animateField = e.target.checked;
            }
        });
        document.getElementById('arrowSize').addEventListener('input', (e) => {
            if (this.vectorFieldRenderer) {
                this.vectorFieldRenderer.arrowScale = parseFloat(e.target.value) / 15;
                this.vectorFieldRenderer.render();
            }
        });

        document.getElementById('alertList').addEventListener('click', (e) => this.handleAlertClick(e));
    }

    initDefaults() {
        setTimeout(async () => {
            await this.calculateDefaultField();
            await this.generateDefaultVectorField();
        }, 1000);
    }

    async calculateField() {
        const lat = parseFloat(document.getElementById('centerLat').value);
        const lon = parseFloat(document.getElementById('centerLon').value);
        const year = parseFloat(document.getElementById('targetYear').value);

        if (isNaN(lat) || isNaN(lon) || isNaN(year)) {
            showToast('请输入有效的坐标和年份', 'warning');
            return;
        }

        const btn = document.getElementById('calcFieldBtn');
        btn.disabled = true;
        btn.textContent = '计算中...';

        try {
            const response = await dataService.calculateGeomagneticField(lat, lon, year);
            this.currentGeomagneticField = response.data;

            if (window.sinan3d) {
                window.sinan3d.setFieldIntensity(this.currentGeomagneticField.field_intensity);
            }

            showToast(`地磁场计算完成: ${this.currentGeomagneticField.field_intensity.toFixed(0)} nT`, 'success');
            console.log('地磁场数据:', this.currentGeomagneticField);
        } catch (e) {
            showToast('地磁场计算失败: ' + e.message, 'error');
        } finally {
            btn.disabled = false;
            btn.textContent = '计算地磁场';
        }
    }

    async calculateDefaultField() {
        try {
            const response = await dataService.calculateGeomagneticField(34.265, 108.955, -100);
            this.currentGeomagneticField = response.data;
            if (window.sinan3d) {
                window.sinan3d.setFieldIntensity(this.currentGeomagneticField.field_intensity);
            }
        } catch (e) {
            console.warn('默认地磁场计算失败:', e);
        }
    }

    async generateVectorField() {
        const targetYear = parseFloat(document.getElementById('targetYear').value);
        const centerLat = parseFloat(document.getElementById('centerLat').value);
        const centerLon = parseFloat(document.getElementById('centerLon').value);
        const gridSize = parseInt(document.getElementById('gridSize').value);

        const request = {
            target_year: targetYear,
            center_lat: centerLat,
            center_lon: centerLon,
            radius_km: 500,
            grid_size: gridSize,
            altitude_km: 0
        };

        const btn = document.getElementById('genVectorBtn');
        btn.disabled = true;
        btn.textContent = '生成中...';

        try {
            const response = await dataService.generateVectorField(request);
            this.vectorFieldRenderer.setData(response.data);

            document.getElementById('legendYear').textContent = targetYear;
            document.getElementById('legendCenter').textContent =
                `${centerLat.toFixed(3)}°N, ${centerLon.toFixed(3)}°E`;
            document.getElementById('legendGrid').textContent = `${gridSize}×${gridSize}`;

            showToast(`矢量场生成完成，共 ${response.data.points.length} 个点`, 'success');
        } catch (e) {
            showToast('矢量场生成失败: ' + e.message, 'error');
        } finally {
            btn.disabled = false;
            btn.textContent = '生成矢量场';
        }
    }

    async generateDefaultVectorField() {
        const request = {
            target_year: -100,
            center_lat: 34.265,
            center_lon: 108.955,
            radius_km: 500,
            grid_size: 15,
            altitude_km: 0
        };

        try {
            const response = await dataService.generateVectorField(request);
            this.vectorFieldRenderer.setData(response.data);
        } catch (e) {
            console.warn('默认矢量场生成失败:', e);
        }
    }

    async runSimulation() {
        const params = {
            device_id: document.getElementById('simDeviceId').value,
            simulation_id: 'SIM-' + Date.now(),
            target_year: parseFloat(document.getElementById('simYear').value),
            location_lat: 34.265,
            location_lon: 108.955,
            magnetic_moment_magnitude: parseFloat(document.getElementById('momentMag').value),
            remanence: parseFloat(document.getElementById('remanence').value),
            temperature: parseFloat(document.getElementById('temperature').value),
            friction_coefficient: parseFloat(document.getElementById('friction').value),
            demagnetization_factor: 0.1,
            anisotropy_constant: 1e4,
            expected_azimuth: 0
        };

        const btn = document.getElementById('runSimulationBtn');
        btn.disabled = true;
        btn.textContent = '仿真中...';

        try {
            const response = await dataService.runPointingSimulation(params);
            const result = response.data;

            showToast(
                `仿真完成! 指向精度: ${result.pointing_accuracy.toFixed(2)}°, ` +
                `仿真方位: ${result.simulated_azimuth.toFixed(2)}°`,
                'success'
            );

            const sensorData = {
                magnetic_moment_x: Math.cos(result.simulated_azimuth * Math.PI / 180) * params.magnetic_moment_magnitude,
                magnetic_moment_y: Math.sin(result.simulated_azimuth * Math.PI / 180) * params.magnetic_moment_magnitude,
                magnetic_moment_z: 0,
                magnetic_moment_magnitude: params.magnetic_moment_magnitude,
                remanence: params.remanence,
                pointing_deviation: Math.abs(result.simulated_azimuth - result.expected_azimuth),
                environment_temp: params.temperature,
                location_lat: params.location_lat,
                location_lon: params.location_lon,
                is_alert: Math.abs(result.simulated_azimuth - result.expected_azimuth) > CONFIG.THRESHOLDS.WARNING,
                timestamp: new Date().toISOString(),
                device_id: params.device_id
            };

            if (window.sinan3d) {
                window.sinan3d.updateSensorData(sensorData);
            }
            this.chartManager.addSensorDataPoint(sensorData);

            this.loadSimulationResults();
        } catch (e) {
            showToast('仿真失败: ' + e.message, 'error');
        } finally {
            btn.disabled = false;
            btn.textContent = '运行仿真';
        }
    }

    async loadSimulationResults() {
        try {
            const response = await dataService.getSimulationResults({ limit: 50 });
            const results = response.data || [];
            this.dataCache.simulationResults = results;

            const tbody = document.getElementById('simulationTableBody');

            if (results.length === 0) {
                tbody.innerHTML = '<tr><td colspan="10" class="no-alerts">暂无仿真结果</td></tr>';
                return;
            }

            tbody.innerHTML = '';

            results.slice(0, 20).forEach(result => {
                const tr = document.createElement('tr');

                let accuracyClass = 'accuracy-high';
                if (result.pointing_accuracy > 1) accuracyClass = 'accuracy-medium';
                if (result.pointing_accuracy > 2) accuracyClass = 'accuracy-low';

                const time = new Date(result.timestamp).toLocaleString('zh-CN', {
                    month: '2-digit',
                    day: '2-digit',
                    hour: '2-digit',
                    minute: '2-digit'
                });

                tr.innerHTML = `
                    <td>${result.simulation_id.slice(-8)}</td>
                    <td>${result.device_id}</td>
                    <td>${result.target_year}</td>
                    <td>${result.expected_azimuth.toFixed(2)}°</td>
                    <td>${result.simulated_azimuth.toFixed(2)}°</td>
                    <td class="${accuracyClass}">${result.pointing_accuracy.toFixed(2)}°</td>
                    <td>${result.magnetic_moment_magnitude.toFixed(4)}</td>
                    <td>${result.remanence.toFixed(3)}</td>
                    <td>${result.temperature.toFixed(1)}°C</td>
                    <td>${time}</td>
                `;

                tbody.appendChild(tr);
            });
        } catch (e) {
            console.error('加载仿真结果失败:', e);
        }
    }

    async loadActiveAlerts() {
        try {
            const response = await dataService.getActiveAlerts(50);
            const alerts = response.data || [];

            const alertList = document.getElementById('alertList');

            if (alerts.length === 0) {
                alertList.innerHTML = '<div class="no-alerts">暂无告警</div>';
                document.getElementById('alertCount').textContent = 0;
                return;
            }

            alertList.innerHTML = '';

            alerts.slice(0, 10).forEach(alert => {
                const div = document.createElement('div');
                div.className = `alert-item ${alert.alert_level === 'WARNING' ? 'warning' : ''}`;
                div.dataset.alertId = alert.id;

                const time = new Date(alert.timestamp).toLocaleTimeString('zh-CN', {
                    hour: '2-digit',
                    minute: '2-digit'
                });

                div.innerHTML = `
                    <div class="alert-item-header">
                        <span class="alert-device">${alert.device_id}</span>
                        <span class="alert-time">${time}</span>
                    </div>
                    <div class="alert-message">${alert.message}</div>
                `;

                alertList.appendChild(div);
            });

            document.getElementById('alertCount').textContent = alerts.length;
        } catch (e) {
            console.error('加载告警失败:', e);
        }
    }

    async handleAlertClick(e) {
        const alertItem = e.target.closest('.alert-item');
        if (!alertItem) return;

        const alertId = alertItem.dataset.alertId;

        try {
            await dataService.acknowledgeAlert(alertId, '前端用户');
            alertItem.remove();
            showToast('告警已确认', 'success');
            this.loadActiveAlerts();
        } catch (e) {
            showToast('确认告警失败', 'error');
        }
    }

    async loadDevices() {
        try {
            const response = await dataService.getDevices();
            const devices = response.devices || [];

            const deviceList = document.getElementById('deviceList');
            deviceList.innerHTML = '';

            if (devices.length === 0) {
                deviceList.innerHTML = '<div class="no-alerts">暂无设备</div>';
                return;
            }

            devices.forEach(device => {
                const div = document.createElement('div');
                div.className = 'device-item';
                div.dataset.deviceId = device.device_id;

                const hasAlert = device.latest_data?.is_alert;
                const deviation = device.latest_data?.pointing_deviation || 0;

                let deviationClass = '';
                if (deviation >= CONFIG.THRESHOLDS.CRITICAL) {
                    deviationClass = 'critical';
                } else if (deviation >= CONFIG.THRESHOLDS.WARNING) {
                    deviationClass = 'warning';
                }

                div.innerHTML = `
                    <div class="device-item-header">
                        <span class="device-id">${device.device_id}</span>
                        <span class="device-status ${hasAlert ? 'alert' : ''}"></span>
                    </div>
                    <div class="device-name">${device.device_name}</div>
                    <div class="device-deviation ${deviationClass}">
                        偏差: <span class="value">${deviation.toFixed(2)}°</span>
                    </div>
                `;

                div.addEventListener('click', () => this.selectDevice(device));
                deviceList.appendChild(div);
            });

            if (devices.length > 0 && !this.selectedDeviceId) {
                this.selectDevice(devices[0]);
            }
        } catch (e) {
            console.error('加载设备失败:', e);
            document.getElementById('deviceList').innerHTML =
                '<div class="no-alerts">加载设备失败</div>';
        }
    }

    selectDevice(device) {
        this.selectedDeviceId = device.device_id;

        document.querySelectorAll('.device-item').forEach(item => {
            item.classList.remove('active');
            if (item.dataset.deviceId === this.selectedDeviceId) {
                item.classList.add('active');
            }
        });

        if (device.latest_data && window.sinan3d) {
            window.sinan3d.updateSensorData(device.latest_data);
        }

        this.loadSensorDataForDevice(this.selectedDeviceId);
    }

    async loadSensorDataForDevice(deviceId) {
        try {
            const response = await dataService.getLatestSensorData(deviceId);
            const data = response.data || [];

            if (data.length > 0) {
                this.dataCache.deviation = data.slice().reverse();
                this.chartManager.dataCache.deviation = this.dataCache.deviation;
                this.chartManager.updateDeviationData(this.dataCache.deviation);
                this.chartManager.updateMomentData(this.dataCache.deviation);
                this.chartManager.updateTemperatureData(this.dataCache.deviation);
            }
        } catch (e) {
            console.error('加载传感器数据失败:', e);
        }
    }

    addSensorDataPoint(data) {
        this.chartManager.addSensorDataPoint(data);
        if (data.device_id === this.selectedDeviceId && window.sinan3d) {
            window.sinan3d.updateSensorData(data);
        }
    }

    resizeVectorField() {
        if (this.vectorFieldRenderer) {
            this.vectorFieldRenderer.resize();
        }
    }

    resizeCharts() {
        if (this.chartManager && this.chartManager.charts) {
            Object.values(this.chartManager.charts).forEach(chart => {
                if (chart) chart.resize();
            });
        }
    }

    destroy() {
        if (this.vectorFieldRenderer) this.vectorFieldRenderer.clear();
        if (this.chartManager) this.chartManager.destroy();
    }
}
