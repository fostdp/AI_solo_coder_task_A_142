class VectorFieldRenderer {
    constructor(canvasId) {
        this.canvas = document.getElementById(canvasId);
        this.ctx = this.canvas.getContext('2d');
        
        this.vectorFieldData = null;
        this.showArrows = true;
        this.showHeatmap = true;
        this.showStreamlines = true;
        this.animateField = false;
        this.arrowSize = 15;
        this.animationTime = 0;
        
        this.minMagnitude = Infinity;
        this.maxMagnitude = -Infinity;
        
        this.animationId = null;
        this.streamlines = [];
        this.streamlineParticles = [];
        
        this.streamlineConfig = {
            stepSize: 5,
            maxSteps: 150,
            separationDistance: 25,
            dithering: 0.3,
            minMagnitudeRatio: 0.15,
            simplificationTolerance: 2.5,
            maxStreamlines: 60,
            lineWidthBase: 1.2,
            lineWidthMagScale: 2.5,
            particleCount: 80,
            particleSpeed: 1.5,
        };
        
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
                this.initStreamlineParticles();
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
            this.computeStreamlines();
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
        
        this.computeStreamlines();
        this.initStreamlineParticles();
        this.render();
    }
    
    buildGridIndex() {
        if (!this.vectorFieldData) return null;
        
        const points = this.vectorFieldData.points;
        const gridSize = this.vectorFieldData.grid_size;
        if (points.length === 0) return null;
        
        const xs = points.map(p => p.x);
        const ys = points.map(p => p.y);
        const minX = Math.min(...xs);
        const maxX = Math.max(...xs);
        const minY = Math.min(...ys);
        const maxY = Math.max(...ys);
        
        const dx = gridSize > 1 ? (maxX - minX) / (gridSize - 1) : 1;
        const dy = gridSize > 1 ? (maxY - minY) / (gridSize - 1) : 1;
        
        return { points, gridSize, minX, maxX, minY, maxY, dx, dy };
    }
    
    interpolateField(px, py, grid) {
        if (!grid) return null;
        
        const { points, gridSize, minX, minY, dx, dy } = grid;
        
        const gi = (px - minX) / dx;
        const gj = (py - minY) / dy;
        
        const i0 = Math.floor(gi);
        const j0 = Math.floor(gj);
        const i1 = Math.min(i0 + 1, gridSize - 1);
        const j1 = Math.min(j0 + 1, gridSize - 1);
        
        if (i0 < 0 || j0 < 0 || i0 >= gridSize || j0 >= gridSize) return null;
        
        const fx = gi - i0;
        const fy = gj - j0;
        
        const idx00 = j0 * gridSize + i0;
        const idx10 = j0 * gridSize + i1;
        const idx01 = j1 * gridSize + i0;
        const idx11 = j1 * gridSize + i1;
        
        if (idx00 >= points.length || idx11 >= points.length) return null;
        
        const p00 = points[idx00];
        const p10 = points[idx10];
        const p01 = points[idx01];
        const p11 = points[idx11];
        
        const bx = (1 - fx) * (1 - fy) * p00.bx + fx * (1 - fy) * p10.bx
                  + (1 - fx) * fy * p01.bx + fx * fy * p11.bx;
        const by = (1 - fx) * (1 - fy) * p00.by + fx * (1 - fy) * p10.by
                  + (1 - fx) * fy * p01.by + fx * fy * p11.by;
        const mag = (1 - fx) * (1 - fy) * p00.magnitude + fx * (1 - fy) * p10.magnitude
                   + (1 - fx) * fy * p01.magnitude + fx * fy * p11.magnitude;
        
        return { bx, by, magnitude: mag };
    }
    
    computeStreamlines() {
        this.streamlines = [];
        const grid = this.buildGridIndex();
        if (!grid) return;
        
        const { points, gridSize, minX, maxX, minY, maxY, dx, dy } = grid;
        const cfg = this.streamlineConfig;
        
        const occupied = new Float32Array(
            Math.ceil(this.width / cfg.separationDistance) *
            Math.ceil(this.height / cfg.separationDistance)
        );
        const occW = Math.ceil(this.width / cfg.separationDistance);
        
        const padding = 50;
        const plotWidth = this.width - padding * 2;
        const plotHeight = this.height - padding * 2;
        const scaleX = plotWidth / (maxX - minX);
        const scaleY = plotHeight / (maxY - minY);
        const scale = Math.min(scaleX, scaleY);
        const offsetX = padding + (plotWidth - (maxX - minX) * scale) / 2;
        const offsetY = padding + (plotHeight - (maxY - minY) * scale) / 2;
        
        const toScreenX = (dataX) => offsetX + (dataX - minX) * scale;
        const toScreenY = (dataY) => offsetY + (dataY - minY) * scale;
        const toDataX = (sx) => (sx - offsetX) / scale + minX;
        const toDataY = (sy) => (sy - offsetY) / scale + minY;
        
        const candidates = [];
        const seedStep = Math.max(2, Math.floor(gridSize / 8));
        for (let j = 1; j < gridSize - 1; j += seedStep) {
            for (let i = 1; i < gridSize - 1; i += seedStep) {
                const idx = j * gridSize + i;
                if (idx < points.length) {
                    const pt = points[idx];
                    const normMag = (pt.magnitude - this.minMagnitude) /
                        (this.maxMagnitude - this.minMagnitude + 1e-10);
                    candidates.push({
                        sx: toScreenX(pt.x),
                        sy: toScreenY(pt.y),
                        dataX: pt.x,
                        dataY: pt.y,
                        priority: normMag + Math.random() * cfg.dithering,
                    });
                }
            }
        }
        candidates.sort((a, b) => b.priority - a.priority);
        
        const isOccupied = (sx, sy) => {
            const ix = Math.floor(sx / cfg.separationDistance);
            const iy = Math.floor(sy / cfg.separationDistance);
            if (ix < 0 || iy < 0 || ix >= occW || iy * occW + ix >= occupied.length) return true;
            return occupied[iy * occW + ix] > 0.5;
        };
        
        const markOccupied = (sx, sy) => {
            const ix = Math.floor(sx / cfg.separationDistance);
            const iy = Math.floor(sy / cfg.separationDistance);
            if (ix >= 0 && iy >= 0 && iy * occW + ix < occupied.length) {
                occupied[iy * occW + ix] = 1.0;
            }
        };
        
        const traceStreamline = (startDataX, startDataY, direction) => {
            const path = [];
            let px = startDataX;
            let py = startDataY;
            
            const dataStepX = cfg.stepSize / scale;
            const dataStepY = cfg.stepSize / scale;
            
            for (let step = 0; step < cfg.maxSteps; step++) {
                const field = this.interpolateField(px, py, grid);
                if (!field) break;
                
                const mag = Math.sqrt(field.bx * field.bx + field.by * field.by);
                if (mag < 1e-10) break;
                
                const normMag = (field.magnitude - this.minMagnitude) /
                    (this.maxMagnitude - this.minMagnitude + 1e-10);
                if (normMag < cfg.minMagnitudeRatio && step > 0) break;
                
                const sx = toScreenX(px);
                const sy = toScreenY(py);
                
                if (sx < offsetX || sx > offsetX + (maxX - minX) * scale ||
                    sy < offsetY || sy > offsetY + (maxY - minY) * scale) {
                    break;
                }
                
                if (step > 0 && isOccupied(sx, sy)) break;
                
                path.push({ sx, sy, magnitude: field.magnitude, normMag });
                markOccupied(sx, sy);
                
                const dirX = direction * field.bx / mag;
                const dirY = direction * field.by / mag;
                
                px += dirX * dataStepX;
                py += dirY * dataStepY;
            }
            
            return path;
        };
        
        let streamlineCount = 0;
        for (const candidate of candidates) {
            if (streamlineCount >= cfg.maxStreamlines) break;
            if (isOccupied(candidate.sx, candidate.sy)) continue;
            
            const forwardPath = traceStreamline(candidate.dataX, candidate.dataY, 1);
            const backwardPath = traceStreamline(candidate.dataX, candidate.dataY, -1);
            
            backwardPath.reverse();
            const fullPath = [...backwardPath, ...forwardPath.slice(1)];
            
            if (fullPath.length < 5) continue;
            
            const simplified = this.douglasPeucker(fullPath, cfg.simplificationTolerance);
            
            if (simplified.length < 3) continue;
            
            const avgMag = fullPath.reduce((s, p) => s + p.normMag, 0) / fullPath.length;
            
            this.streamlines.push({
                points: simplified,
                rawPoints: fullPath,
                avgMagnitude: avgMag,
                length: fullPath.length,
            });
            
            streamlineCount++;
        }
    }
    
    douglasPeucker(points, tolerance) {
        if (points.length <= 2) return points;
        
        let maxDist = 0;
        let maxIdx = 0;
        
        const first = points[0];
        const last = points[points.length - 1];
        
        for (let i = 1; i < points.length - 1; i++) {
            const dist = this.perpendicularDistance(points[i], first, last);
            if (dist > maxDist) {
                maxDist = dist;
                maxIdx = i;
            }
        }
        
        if (maxDist > tolerance) {
            const left = this.douglasPeucker(points.slice(0, maxIdx + 1), tolerance);
            const right = this.douglasPeucker(points.slice(maxIdx), tolerance);
            return [...left.slice(0, -1), ...right];
        } else {
            return [first, last];
        }
    }
    
    perpendicularDistance(point, lineStart, lineEnd) {
        const dx = lineEnd.sx - lineStart.sx;
        const dy = lineEnd.sy - lineStart.sy;
        const lenSq = dx * dx + dy * dy;
        
        if (lenSq < 1e-10) {
            return Math.sqrt(
                (point.sx - lineStart.sx) ** 2 + (point.sy - lineStart.sy) ** 2
            );
        }
        
        const num = Math.abs(
            dy * point.sx - dx * point.sy + lineEnd.sx * lineStart.sy - lineEnd.sy * lineStart.sx
        );
        
        return num / Math.sqrt(lenSq);
    }
    
    initStreamlineParticles() {
        this.streamlineParticles = [];
        if (this.streamlines.length === 0) return;
        
        const cfg = this.streamlineConfig;
        const totalSlots = cfg.particleCount;
        
        for (let i = 0; i < totalSlots; i++) {
            const slIdx = Math.floor(Math.random() * this.streamlines.length);
            const sl = this.streamlines[slIdx];
            if (sl.rawPoints.length < 2) continue;
            
            const t = Math.random();
            this.streamlineParticles.push({
                streamlineIndex: slIdx,
                t: t,
                speed: cfg.particleSpeed * (0.7 + Math.random() * 0.6),
                opacity: 0.5 + Math.random() * 0.5,
            });
        }
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
        
        if (this.showStreamlines && this.streamlines.length > 0) {
            this.drawStreamlines();
        } else if (this.showArrows) {
            this.drawArrows(points, minX, minY, scale, offsetX, offsetY);
        }
        
        if (this.animateField && this.streamlines.length > 0) {
            this.drawStreamlineParticles();
        }
        
        this.drawLabels(minX, maxX, minY, maxY);
    }
    
    drawStreamlines() {
        const cfg = this.streamlineConfig;
        
        for (const sl of this.streamlines) {
            const pts = sl.points;
            if (pts.length < 2) continue;
            
            const baseWidth = cfg.lineWidthBase + sl.avgMagnitude * cfg.lineWidthMagScale;
            
            this.ctx.beginPath();
            this.ctx.moveTo(pts[0].sx, pts[0].sy);
            
            for (let i = 1; i < pts.length; i++) {
                if (i < pts.length - 1) {
                    const cpx = (pts[i].sx + pts[i + 1].sx) / 2;
                    const cpy = (pts[i].sy + pts[i + 1].sy) / 2;
                    this.ctx.quadraticCurveTo(pts[i].sx, pts[i].sy, cpx, cpy);
                } else {
                    this.ctx.lineTo(pts[i].sx, pts[i].sy);
                }
            }
            
            this.ctx.strokeStyle = this.getColorForMagnitude(
                this.minMagnitude + sl.avgMagnitude * (this.maxMagnitude - this.minMagnitude),
                0.75
            );
            this.ctx.lineWidth = baseWidth;
            this.ctx.lineCap = 'round';
            this.ctx.lineJoin = 'round';
            this.ctx.stroke();
            
            const lastPt = pts[pts.length - 1];
            const prevPt = pts[Math.max(0, pts.length - 3)];
            const angle = Math.atan2(lastPt.sy - prevPt.sy, lastPt.sx - prevPt.sx);
            const headLen = 4 + sl.avgMagnitude * 5;
            const headAngle = Math.PI / 7;
            
            this.ctx.beginPath();
            this.ctx.moveTo(lastPt.sx, lastPt.sy);
            this.ctx.lineTo(
                lastPt.sx - headLen * Math.cos(angle - headAngle),
                lastPt.sy - headLen * Math.sin(angle - headAngle)
            );
            this.ctx.lineTo(
                lastPt.sx - headLen * Math.cos(angle + headAngle),
                lastPt.sy - headLen * Math.sin(angle + headAngle)
            );
            this.ctx.closePath();
            this.ctx.fillStyle = this.getColorForMagnitude(
                this.minMagnitude + sl.avgMagnitude * (this.maxMagnitude - this.minMagnitude),
                0.85
            );
            this.ctx.fill();
        }
    }
    
    drawStreamlineParticles() {
        for (const particle of this.streamlineParticles) {
            const sl = this.streamlines[particle.streamlineIndex];
            if (!sl || sl.rawPoints.length < 2) continue;
            
            const pts = sl.rawPoints;
            const totalLen = this.computePathLength(pts);
            const dist = particle.t * totalLen;
            const pos = this.getPointAtDistance(pts, dist);
            
            if (!pos) continue;
            
            const normMag = pos.normMag || 0.5;
            const particleSize = 2 + normMag * 4;
            
            this.ctx.beginPath();
            this.ctx.arc(pos.sx, pos.sy, particleSize, 0, Math.PI * 2);
            this.ctx.fillStyle = this.getColorForMagnitude(
                this.minMagnitude + normMag * (this.maxMagnitude - this.minMagnitude),
                particle.opacity * 0.9
            );
            this.ctx.fill();
            
            this.ctx.shadowColor = this.getColorForMagnitude(
                this.minMagnitude + normMag * (this.maxMagnitude - this.minMagnitude),
                1
            );
            this.ctx.shadowBlur = 8;
            this.ctx.fill();
            this.ctx.shadowBlur = 0;
        }
    }
    
    computePathLength(pts) {
        let len = 0;
        for (let i = 1; i < pts.length; i++) {
            const dx = pts[i].sx - pts[i - 1].sx;
            const dy = pts[i].sy - pts[i - 1].sy;
            len += Math.sqrt(dx * dx + dy * dy);
        }
        return len;
    }
    
    getPointAtDistance(pts, targetDist) {
        let accumulated = 0;
        for (let i = 1; i < pts.length; i++) {
            const dx = pts[i].sx - pts[i - 1].sx;
            const dy = pts[i].sy - pts[i - 1].sy;
            const segLen = Math.sqrt(dx * dx + dy * dy);
            
            if (accumulated + segLen >= targetDist) {
                const t = segLen > 0 ? (targetDist - accumulated) / segLen : 0;
                return {
                    sx: pts[i - 1].sx + dx * t,
                    sy: pts[i - 1].sy + dy * t,
                    normMag: pts[i - 1].normMag + (pts[i].normMag - pts[i - 1].normMag) * t,
                };
            }
            accumulated += segLen;
        }
        return pts[pts.length - 1];
    }
    
    updateParticles() {
        for (const particle of this.streamlineParticles) {
            particle.t += particle.speed * 0.003;
            if (particle.t > 1.0) {
                particle.t = 0;
                particle.streamlineIndex = Math.floor(Math.random() * this.streamlines.length);
                particle.speed = this.streamlineConfig.particleSpeed * (0.7 + Math.random() * 0.6);
            }
        }
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
                this.updateParticles();
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
        this.streamlines = [];
        this.streamlineParticles = [];
        this.stopAnimation();
    }
}
