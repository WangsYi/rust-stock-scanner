# AKShare Python Service Dockerfile
FROM python:3.11-slim

# Install system dependencies
RUN apt-get update && apt-get install -y \
    gcc \
    g++ \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy requirements and install Python dependencies
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Copy application code
COPY akshare_service.py .
COPY setup_akshare_service.sh .

# Create cache directory
RUN mkdir -p /app/cache && \
    chmod 755 /app/cache

# Create non-root user
RUN useradd -m -u 1000 akshareuser && \
    chown -R akshareuser:akshareuser /app

# Switch to non-root user
USER akshareuser

# Expose port
EXPOSE 5000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:5000/health || exit 1

# Set environment variables
ENV FLASK_APP=akshare_service.py \
    FLASK_ENV=production \
    PYTHONPATH=/app

# Run the application
CMD ["python", "akshare_service.py"]