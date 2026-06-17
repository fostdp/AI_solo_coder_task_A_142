class VectorFieldRenderer {
    constructor(canvasId) {
        this.canvas = document.getElementById(canvasId);
        this.ctx = this.canvas.getContext('2d');
        
        this.vectorFieldData = null;
        this.showArrows = true;
        this.showHeatmap = true;
        this.animateField = false;
        this.arrowSize = 15;
        this.animationTime = 0;
        
        this.minMagnitude = Infinity;
        this.maxMagnitude = -Infinity;
        
        this.animationId = null;
        
        this.init();
    }
    
    init() {
        this.resize();
        window.addEventListener('resize', () => this.resize());
        
        document.getElementById('showArrows')?.addEventListener('change', (e) => {
            this.showArrows = e.target.checked;
            this.render();
        });
        
        document.getElementById('showHeatmap')?.addEventListener('change', (e) => {
            this.showHeatmap = e.target.checked;
            this.render();
        });
        
        document.getElementById('animateField')?.addEventListener('change', (e) => {
            this.animateField = e.target.checked;
            if (this.animateField) {
                this.startAnimation();
            } else {
                this.stopAnimation();
            }
        });
        
        document.getElementById('arrowSize')?.addEventListener('input', (e) => {
            this.arrowSize = parseInt(e.target.value);
            this.render();
        });
    }
    
    resize() {
        const rect = this.canvas.parentElement.getBoundingClientRect();
        const dpr = window.devicePixelRatio || 1;
        
        this.canvas.width = rect.width * dpr;
        this.canvas.height = rect.height * dpr;
        this.canvas.style.width = rect.width + 'px';
        this.canvas.style.height = rect.height + 'px';
        
        this.ctx.scale(dpr, dpr);
        this.width = rect.width;
        this.height = rect.height;
        
        if (this.vectorFieldData) {
            this.render();
        }
    }
    
    setData(data) {
        this.vectorFieldData = data;
        
        this.minMagnitude = Infinity;
        this.maxMagnitude = -Infinity;
        
        data.points.forEach(point => {
            this.minMagnitude = Math.min(this.minMagnitude, point.magnitude);
            this.maxMagnitude = Math.max(this.maxMagnitude, point.magnitude);
        });
        
        document.getElementById('legendYear').textContent = data.target_year;
        document.getElementById('legendCenter').textContent = 
            `${data.center_lat.toFixed(3)}°N, ${data.center_lon.toFixed(3)}°E`;
        document.getElementById('legendGrid').textContent = 
            `${data.grid_size}×${data.grid_size}`;
        
        this.render();
    }
    
    getColorForMagnitude(magnitude, alpha = 1) {
        const normalized = (magnitude - this.minMagnitude) / (this.maxMagnitude - this.minMagnitude);
        
        const r = Math.floor(30 + normalized * 200);
        const g = Math.floor(50 + (1 - normalized) * 100);
        const b = Math.floor(255 - normalized * 150);
        
        return `rgba(${r}, ${g}, ${b}, ${alpha})`;
    }
    
    render() {
        if (!this.vectorFieldData || !this.vectorFieldData.points) return;
        
        this.ctx.clearRect(0, 0, this.width, this.height);
        
        const padding = 50;
        const plotWidth = this.width - padding * 2;
        const plotHeight = this.height - padding * 2;
        
        const points = this.vectorFieldData.points;
        const gridSize = this.vectorFieldData.grid_size;
        
        const minX = Math.min(...points.map(p => p.x));
        const maxX = Math.max(...points.map(p => p.x));
        const minY = Math.min(...points.map(p => p.y));
        const maxY = Math.max(...points.map(p => p.y));
        
        const scaleX = plotWidth / (maxX - minX);
        const scaleY = plotHeight / (maxY - minY);
        const scale = Math.min(scaleX, scaleY);
        
        const offsetX = padding + (plotWidth - (maxX - minX) * scale) / 2;
        const offsetY = padding + (plotHeight - (maxY - minY) * scale) / 2;
        
        if (this.showHeatmap) {
            this.drawHeatmap(points, minX, minY, scale, offsetX, offsetY, gridSize);
        }
        
        this.drawGrid(minX, maxX, minY, maxY, scale, offsetX, offsetY);
        
        if (this.showArrows) {
            this.drawArrows(points, minX, minY, scale, offsetX, offsetY);
        }
        
        this.drawLabels(minX, maxX, minY, maxY);
    }
    
    drawHeatmap(points, minX, minY, scale, offsetX, offsetY, gridSize) {
        const cellWidth = (points[1]?.x - points[0]?.x || 1) * scale;
        const cellHeight = (points[gridSize]?.y - points[0]?.y || 1) * scale;
        
        for (let i = 0; i < points.length; i++) {
            const point = points[i];
            const x = offsetX + (point.x - minX) * scale - cellWidth / 2;
            const y = offsetY + (point.y - minY) * scale - cellHeight / 2;
            
            const color = this.getColorForMagnitude(point.magnitude, 0.4);
            
            this.ctx.fillStyle = color;
            this.ctx.fillRect(x, y, cellWidth + 1, cellHeight + 1);
        }
        
        if (this.animateField) {
            this.drawAnimatedParticles(points, minX, minY, scale, offsetX, offsetY);
        }
    }
    
    drawAnimatedParticles(points, minX, minY, scale, offsetX, offsetY) {
        const particleCount = 50;
        
        for (let i = 0; i < particleCount; i++) {
            const t = (this.animationTime * 0.001 + i * 0.1) % 1;
            const pointIndex = Math.floor(t * (points.length - 1));
            const point = points[pointIndex];
            const nextPoint = points[Math.min(pointIndex + 1, points.length - 1)];
            
            const localT = (t * (points.length - 1)) % 1;
            
            const x = offsetX + (point.x + (nextPoint.x - point.x) * localT - minX) * scale;
            const y = offsetY + (point.y + (nextPoint.y - point.y) * localT - minY) * scale;
            
            const bx = point.bx + (nextPoint.bx - point.bx) * localT;
            const by = point.by + (nextPoint.by - point.by) * localT;
            
            const angle = Math.atan2(by, bx);
            const magnitude = Math.sqrt(bx * bx + by * by);
            const normalizedMag = (magnitude - this.minMagnitude) / (this.maxMagnitude - this.minMagnitude);
            
            const particleSize = 2 + normalizedMag * 4;
            
            this.ctx.beginPath();
            this.ctx.arc(x, y, particleSize, 0, Math.PI * 2);
            this.ctx.fillStyle = this.getColorForMagnitude(magnitude, 0.8);
            this.ctx.fill();
            
            this.ctx.shadowColor = this.getColorForMagnitude(magnitude, 1);
            this.ctx.shadowBlur = 10;
            this.ctx.fill();
            this.ctx.shadowBlur = 0;
        }
    }
    
    drawArrows(points, minX, minY, scale, offsetX, offsetY) {
        const arrowScale = this.arrowSize / 15;
        
        for (const point of points) {
            const x = offsetX + (point.x - minX) * scale;
            const y = offsetY + (point.y - minY) * scale;
            
            const angle = Math.atan2(point.by, point.bx);
            const magnitude = Math.sqrt(point.bx * point.bx + point.by * point.by);
            
            const normalizedMag = (magnitude - this.minMagnitude) / (this.maxMagnitude - this.minMagnitude);
            const arrowLength = 10 + normalizedMag * 20 * arrowScale;
            
            const endX = x + Math.cos(angle) * arrowLength;
            const endY = y + Math.sin(angle) * arrowLength;
            
            this.ctx.beginPath();
            this.ctx.moveTo(x, y);
            this.ctx.lineTo(endX, endY);
            this.ctx.strokeStyle = this.getColorForMagnitude(magnitude, 0.9);
            this.ctx.lineWidth = 1.5 + normalizedMag * 2;
            this.ctx.stroke();
            
            const headLength = 5 * arrowScale;
            const headAngle = Math.PI / 6;
            
            this.ctx.beginPath();
            this.ctx.moveTo(endX, endY);
            this.ctx.lineTo(
                endX - headLength * Math.cos(angle - headAngle),
                endY - headLength * Math.sin(angle - headAngle)
            );
            this.ctx.lineTo(
                endX - headLength * Math.cos(angle + headAngle),
                endY - headLength * Math.sin(angle + headAngle)
            );
            this.ctx.closePath();
            this.ctx.fillStyle = this.getColorForMagnitude(magnitude, 0.9);
            this.ctx.fill();
        }
    }
    
    drawGrid(minX, maxX, minY, maxY, scale, offsetX, offsetY) {
        this.ctx.strokeStyle = 'rgba(100, 150, 255, 0.2)';
        this.ctx.lineWidth = 1;
        
        const xStep = Math.ceil((maxX - minX) / 5 / 100) * 100;
        for (let x = Math.floor(minX / xStep) * xStep; x <= maxX; x += xStep) {
            const screenX = offsetX + (x - minX) * scale;
            this.ctx.beginPath();
            this.ctx.moveTo(screenX, offsetY);
            this.ctx.lineTo(screenX, offsetY + (maxY - minY) * scale);
            this.ctx.stroke();
        }
        
        const yStep = Math.ceil((maxY - minY) / 5 / 100) * 100;
        for (let y = Math.floor(minY / yStep) * yStep; y <= maxY; y += yStep) {
            const screenY = offsetY + (y - minY) * scale;
            this.ctx.beginPath();
            this.ctx.moveTo(offsetX, screenY);
            this.ctx.lineTo(offsetX + (maxX - minX) * scale, screenY);
            this.ctx.stroke();
        }
        
        this.ctx.strokeStyle = 'rgba(100, 150, 255, 0.5)';
        this.ctx.lineWidth = 2;
        this.ctx.strokeRect(
            offsetX, offsetY,
            (maxX - minX) * scale,
            (maxY - minY) * scale
        );
    }
    
    drawLabels(minX, maxX, minY, maxY) {
        this.ctx.fillStyle = 'rgba(200, 200, 200, 0.8)';
        this.ctx.font = '12px Microsoft YaHei';
        this.ctx.textAlign = 'center';
        
        this.ctx.fillText(`东向 (km)`, this.width / 2, this.height - 20);
        
        this.ctx.save();
        this.ctx.translate(20, this.height / 2);
        this.ctx.rotate(-Math.PI / 2);
        this.ctx.fillText(`北向 (km)`, 0, 0);
        this.ctx.restore();
        
        this.ctx.fillStyle = 'rgba(100, 180, 255, 0.9)';
        this.ctx.font = '10px Microsoft YaHei';
        this.ctx.textAlign = 'left';
        
        const padding = 50;
        const plotWidth = this.width - padding * 2;
        const plotHeight = this.height - padding * 2;
        
        this.ctx.fillText(`${minX.toFixed(0)}`, padding, this.height - padding + 15);
        this.ctx.textAlign = 'right';
        this.ctx.fillText(`${maxX.toFixed(0)}`, this.width - padding, this.height - padding + 15);
        
        this.ctx.textAlign = 'right';
        this.ctx.fillText(`${maxY.toFixed(0)}`, padding - 5, padding + 10);
        this.ctx.fillText(`${minY.toFixed(0)}`, padding - 5, this.height - padding);
    }
    
    startAnimation() {
        const animate = () => {
            this.animationTime++;
            if (this.animateField && this.vectorFieldData) {
                this.render();
            }
            this.animationId = requestAnimationFrame(animate);
        };
        animate();
    }
    
    stopAnimation() {
        if (this.animationId) {
            cancelAnimationFrame(this.animationId);
            this.animationId = null;
        }
        if (this.vectorFieldData) {
            this.render();
        }
    }
    
    clear() {
        this.ctx.clearRect(0, 0, this.width, this.height);
        this.vectorFieldData = null;
        this.stopAnimation();
    }
}
