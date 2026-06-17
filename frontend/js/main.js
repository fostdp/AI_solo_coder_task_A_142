let sinan3d = null;
let vectorFieldRenderer = null;
let chartManager = null;
let selectedDeviceId = null;
let currentGeomagneticField = null;

document.addEventListener('DOMContentLoaded', () => {
    initApp();
});

async function initApp() {
    try {
        await dataService.getHealth();
        showToast('后端服务连接成功', 'success');
    } catch (e) {
        showToast('无法连接后端服务，请检查服务是否启动', 'error');
    }

    sinan3d = new Sinan3D('sinanCanvas');
    vectorFieldRenderer = new VectorFieldRenderer('vectorFieldCanvas');
    chartManager = new ChartManager();
    chartManager.init();

    setupTabNavigation();
    setupEventListeners();
    setupGridSlider();

    await loadDevices();
    await loadStatistics();
    await loadActiveAlerts();
    await loadSimulationResults();

    startDataUpdates();
    startSensorStream();

    setTimeout(async () => {
        await calculateDefaultField();
        await generateDefaultVectorField();
    }, 1000);
}

function setupTabNavigation() {
    const tabButtons = document.querySelectorAll('.tab-btn');
    
    tabButtons.forEach(btn => {
        btn.addEventListener('click', () => {
            const targetView = btn.dataset.view;
            
            tabButtons.forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            
            document.querySelectorAll('.view').forEach(view => {
                view.classList.remove('active');
            });
            document.getElementById(targetView + 'View').classList.add('active');
            
            if (targetView === 'vectorfield') {
                setTimeout(() => vectorFieldRenderer.resize(), 100);
            } else if (targetView === 'charts') {
                Object.values(chartManager.charts).forEach(chart => {
                    if (chart) chart.resize();
                });
            }
        });
    });
}

function setupEventListeners() {
    document.getElementById('calcFieldBtn').addEventListener('click', calculateField);
    document.getElementById('genVectorBtn').addEventListener('click', generateVectorField);
    document.getElementById('runSimulationBtn').addEventListener('click', runSimulation);
    document.getElementById('refreshSimBtn').addEventListener('click', loadSimulationResults);
    
    document.getElementById('alertList').addEventListener('click', handleAlertClick);
}

function setupGridSlider() {
    const slider = document.getElementById('gridSize');
    const valueSpan = document.getElementById('gridSizeValue');
    
    slider.addEventListener('input', () => {
        valueSpan.textContent = slider.value;
    });
}

async function loadDevices() {
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
            
            div.addEventListener('click', () => selectDevice(device));
            deviceList.appendChild(div);
        });
        
        if (devices.length > 0 && !selectedDeviceId) {
            selectDevice(devices[0]);
        }
    } catch (e) {
        console.error('加载设备失败:', e);
        document.getElementById('deviceList').innerHTML = 
            '<div class="no-alerts">加载设备失败</div>';
    }
}

function selectDevice(device) {
    selectedDeviceId = device.device_id;
    
    document.querySelectorAll('.device-item').forEach(item => {
        item.classList.remove('active');
        if (item.dataset.deviceId === selectedDeviceId) {
            item.classList.add('active');
        }
    });
    
    if (device.latest_data) {
        sinan3d.updateSensorData(device.latest_data);
    }
    
    loadSensorDataForDevice(selectedDeviceId);
}

async function loadSensorDataForDevice(deviceId) {
    try {
        const response = await dataService.getLatestSensorData(deviceId);
        const data = response.data || [];
        
        if (data.length > 0) {
            chartManager.dataCache.deviation = data.slice().reverse();
            chartManager.updateDeviationData(chartManager.dataCache.deviation);
            chartManager.updateMomentData(chartManager.dataCache.deviation);
            chartManager.updateTemperatureData(chartManager.dataCache.deviation);
        }
    } catch (e) {
        console.error('加载传感器数据失败:', e);
    }
}

async function loadStatistics() {
    try {
        const response = await dataService.getStatistics();
        const stats = response.data || {};
        
        document.getElementById('deviceCount').textContent = stats.total_sensor_records ? '在线' : '0';
        document.getElementById('alertCount').textContent = stats.active_alerts || 0;
        document.getElementById('avgDeviation').textContent = 
            (stats.average_deviation_last_hour || 0).toFixed(2) + '°';
    } catch (e) {
        console.error('加载统计数据失败:', e);
    }
}

async function loadActiveAlerts() {
    try {
        const response = await dataService.getActiveAlerts(50);
        const alerts = response.data || [];
        
        const alertList = document.getElementById('alertList');
        
        if (alerts.length === 0) {
            alertList.innerHTML = '<div class="no-alerts">暂无告警</div>';
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

async function handleAlertClick(e) {
    const alertItem = e.target.closest('.alert-item');
    if (!alertItem) return;
    
    const alertId = alertItem.dataset.alertId;
    
    try {
        await dataService.acknowledgeAlert(alertId, '前端用户');
        alertItem.remove();
        showToast('告警已确认', 'success');
        loadActiveAlerts();
    } catch (e) {
        showToast('确认告警失败', 'error');
    }
}

async function calculateField() {
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
        currentGeomagneticField = response.data;
        
        sinan3d.setFieldIntensity(currentGeomagneticField.field_intensity);
        
        showToast(`地磁场计算完成: ${currentGeomagneticField.field_intensity.toFixed(0)} nT`, 'success');
        console.log('地磁场数据:', currentGeomagneticField);
    } catch (e) {
        showToast('地磁场计算失败: ' + e.message, 'error');
    } finally {
        btn.disabled = false;
        btn.textContent = '计算地磁场';
    }
}

async function calculateDefaultField() {
    try {
        const response = await dataService.calculateGeomagneticField(34.265, 108.955, -100);
        currentGeomagneticField = response.data;
        sinan3d.setFieldIntensity(currentGeomagneticField.field_intensity);
    } catch (e) {
        console.warn('默认地磁场计算失败:', e);
    }
}

async function generateVectorField() {
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
        vectorFieldRenderer.setData(response.data);
        
        showToast(`矢量场生成完成，共 ${response.data.points.length} 个点`, 'success');
    } catch (e) {
        showToast('矢量场生成失败: ' + e.message, 'error');
    } finally {
        btn.disabled = false;
        btn.textContent = '生成矢量场';
    }
}

async function generateDefaultVectorField() {
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
        vectorFieldRenderer.setData(response.data);
    } catch (e) {
        console.warn('默认矢量场生成失败:', e);
    }
}

async function runSimulation() {
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
        
        sinan3d.updateSensorData(sensorData);
        chartManager.addSensorDataPoint(sensorData);
        
        loadSimulationResults();
    } catch (e) {
        showToast('仿真失败: ' + e.message, 'error');
    } finally {
        btn.disabled = false;
        btn.textContent = '运行仿真';
    }
}

async function loadSimulationResults() {
    try {
        const response = await dataService.getSimulationResults({ limit: 50 });
        const results = response.data || [];
        
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

function startDataUpdates() {
    setInterval(() => {
        loadDevices();
        loadStatistics();
    }, CONFIG.UPDATE_INTERVALS.DEVICES);
    
    setInterval(() => {
        loadActiveAlerts();
    }, CONFIG.UPDATE_INTERVALS.ALERTS);
}

function startSensorStream() {
    dataService.startSensorStream(
        (data) => {
            if (Array.isArray(data) && data.length > 0) {
                data.forEach(sensorData => {
                    if (sensorData.device_id === selectedDeviceId) {
                        sinan3d.updateSensorData(sensorData);
                    }
                    chartManager.addSensorDataPoint(sensorData);
                });
            }
        },
        (error) => {
            console.warn('SSE流错误:', error);
        }
    );
}

window.addEventListener('beforeunload', () => {
    dataService.closeSensorStream();
    if (sinan3d) sinan3d.destroy();
    if (vectorFieldRenderer) vectorFieldRenderer.clear();
    if (chartManager) chartManager.destroy();
});
