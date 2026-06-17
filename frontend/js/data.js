class DataService {
    constructor() {
        this.baseUrl = CONFIG.API_BASE_URL;
        this.eventSource = null;
        this.listeners = new Map();
    }
    
    async request(endpoint, options = {}) {
        const url = this.baseUrl + endpoint;
        
        const defaultOptions = {
            headers: {
                'Content-Type': 'application/json',
            },
        };
        
        try {
            const response = await fetch(url, { ...defaultOptions, ...options });
            
            if (!response.ok) {
                const error = await response.json().catch(() => ({ error: 'HTTP Error' }));
                throw new Error(error.error || `HTTP ${response.status}`);
            }
            
            return await response.json();
        } catch (error) {
            console.error(`请求失败 [${endpoint}]:`, error);
            throw error;
        }
    }
    
    async getHealth() {
        return this.request(CONFIG.ENDPOINTS.HEALTH);
    }
    
    async sendSensorData(data) {
        return this.request(CONFIG.ENDPOINTS.SENSOR_DATA, {
            method: 'POST',
            body: JSON.stringify(data),
        });
    }
    
    async getSensorData(params = {}) {
        const query = new URLSearchParams(params).toString();
        const endpoint = query 
            ? `${CONFIG.ENDPOINTS.SENSOR_DATA_QUERY}?${query}`
            : CONFIG.ENDPOINTS.SENSOR_DATA_QUERY;
        return this.request(endpoint);
    }
    
    async getLatestSensorData(deviceId = null) {
        const params = deviceId ? { device_id: deviceId } : {};
        const query = new URLSearchParams(params).toString();
        const endpoint = query
            ? `${CONFIG.ENDPOINTS.SENSOR_LATEST}?${query}`
            : CONFIG.ENDPOINTS.SENSOR_LATEST;
        return this.request(endpoint);
    }
    
    async getDevices() {
        return this.request(CONFIG.ENDPOINTS.DEVICES);
    }
    
    async getDeviceStatus(deviceId) {
        const query = new URLSearchParams({ device_id: deviceId }).toString();
        return this.request(`${CONFIG.ENDPOINTS.DEVICE_STATUS}?${query}`);
    }
    
    async calculateGeomagneticField(lat, lon, year, altitude = 0) {
        const params = { lat, lon, year, altitude };
        const query = new URLSearchParams(params).toString();
        return this.request(`${CONFIG.ENDPOINTS.GEOMAGNETIC_FIELD}?${query}`);
    }
    
    async generateVectorField(request) {
        return this.request(CONFIG.ENDPOINTS.VECTOR_FIELD, {
            method: 'POST',
            body: JSON.stringify(request),
        });
    }
    
    async getSecularVariation(lat, lon, year) {
        const params = { lat, lon, year };
        const query = new URLSearchParams(params).toString();
        return this.request(`${CONFIG.ENDPOINTS.SECULAR_VARIATION}?${query}`);
    }
    
    async runPointingSimulation(params) {
        return this.request(CONFIG.ENDPOINTS.SIMULATION_POINTING, {
            method: 'POST',
            body: JSON.stringify(params),
        });
    }
    
    async getSimulationResults(params = {}) {
        const query = new URLSearchParams(params).toString();
        const endpoint = query
            ? `${CONFIG.ENDPOINTS.SIMULATION_RESULTS}?${query}`
            : CONFIG.ENDPOINTS.SIMULATION_RESULTS;
        return this.request(endpoint);
    }
    
    async getActiveAlerts(limit = 50) {
        const query = new URLSearchParams({ limit }).toString();
        return this.request(`${CONFIG.ENDPOINTS.ALERTS_ACTIVE}?${query}`);
    }
    
    async acknowledgeAlert(alertId, acknowledgedBy, note = null) {
        return this.request(CONFIG.ENDPOINTS.ALERTS_ACKNOWLEDGE, {
            method: 'POST',
            body: JSON.stringify({
                alert_id: alertId,
                acknowledged_by: acknowledgedBy,
                note,
            }),
        });
    }
    
    async getStatistics() {
        return this.request(CONFIG.ENDPOINTS.STATISTICS);
    }
    
    startSensorStream(onData, onError) {
        if (this.eventSource) {
            this.closeSensorStream();
        }
        
        const url = this.baseUrl + CONFIG.ENDPOINTS.SENSOR_STREAM;
        
        try {
            this.eventSource = new EventSource(url);
            
            this.eventSource.onmessage = (event) => {
                try {
                    const data = JSON.parse(event.data);
                    onData(data);
                } catch (e) {
                    console.warn('解析SSE数据失败:', e);
                }
            };
            
            this.eventSource.onerror = (error) => {
                console.warn('SSE连接错误:', error);
                if (onError) onError(error);
                
                if (this.eventSource.readyState === EventSource.CLOSED) {
                    console.info('尝试重新连接SSE...');
                    setTimeout(() => this.startSensorStream(onData, onError), 3000);
                }
            };
            
            this.eventSource.onopen = () => {
                console.info('SSE数据流已连接');
            };
            
            return true;
        } catch (error) {
            console.error('创建SSE连接失败:', error);
            return false;
        }
    }
    
    closeSensorStream() {
        if (this.eventSource) {
            this.eventSource.close();
            this.eventSource = null;
            console.info('SSE数据流已关闭');
        }
    }
    
    on(event, callback) {
        if (!this.listeners.has(event)) {
            this.listeners.set(event, []);
        }
        this.listeners.get(event).push(callback);
    }
    
    off(event, callback) {
        const callbacks = this.listeners.get(event);
        if (callbacks) {
            const index = callbacks.indexOf(callback);
            if (index > -1) {
                callbacks.splice(index, 1);
            }
        }
    }
    
    emit(event, data) {
        const callbacks = this.listeners.get(event);
        if (callbacks) {
            callbacks.forEach(callback => callback(data));
        }
    }
}

const dataService = new DataService();

function showToast(message, type = 'info', duration = 3000) {
    const container = document.getElementById('toastContainer');
    if (!container) return;
    
    const toast = document.createElement('div');
    toast.className = `toast ${type}`;
    toast.textContent = message;
    
    container.appendChild(toast);
    
    setTimeout(() => {
        toast.style.opacity = '0';
        toast.style.transform = 'translateX(100%)';
        toast.style.transition = 'all 0.3s';
        setTimeout(() => toast.remove(), 300);
    }, duration);
}
