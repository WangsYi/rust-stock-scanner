# Akshare Integration Guide

This guide explains how to integrate real akshare data into the Go webapp.

## 🚀 Quick Start

### Option 1: Use Python HTTP Service (Recommended)

1. **Install dependencies**:
   ```bash
   pip install akshare flask flask-cors pandas
   ```

2. **Start the akshare service**:
   ```bash
   cd /home/wangs/code/stock-scanner
   python3 akshare_service.py
   ```

3. **The Go webapp will automatically use real data** from `http://localhost:5000`

### Option 2: Use Setup Script

```bash
cd /home/wangs/code/stock-scanner
./setup_akshare_service.sh
```

## 📊 Data Sources

The akshare integration provides real data for:

- **Stock Price Data**: Historical OHLCV data
- **Fundamental Data**: Financial indicators, ratios, and metrics
- **News Data**: Stock news with sentiment analysis
- **Stock Names**: Company names from stock codes

## 🔧 Configuration

You can configure the akshare proxy endpoint in `config/config.json`:

```json
{
  "akshare": {
    "proxy_url": "http://localhost:5000",
    "timeout": 30
  }
}
```

## 🛠️ API Endpoints

### Python Service (Port 5000)

- **GET** `/api/stock/{code}/price?days=30` - Historical price data
- **GET** `/api/stock/{code}/fundamental` - Financial indicators
- **GET** `/api/stock/{code}/news?days=15` - News and sentiment
- **GET** `/api/stock/{code}/name` - Company name
- **GET** `/health` - Health check

### Go Webapp (Port 8080)

- **POST** `/api/analyze` - Single stock analysis
- **POST** `/api/batch/analyze` - Batch analysis
- **GET** `/api/batch/status/{taskId}` - Progress/status

## 📈 Example Usage

### Single Stock Analysis
```bash
curl -X POST http://localhost:8080/api/analyze \
  -H "Content-Type: application/json" \
  -d '{"stock_code": "000001", "enable_ai": true}'
```

### Batch Analysis
```bash
curl -X POST http://localhost:8080/api/batch/analyze \
  -H "Content-Type: application/json" \
  -d '{"stock_codes": ["000001", "600036", "300019"], "enable_ai": true}'
```

## 🔍 Testing

### Test akshare service
```bash
curl http://localhost:5000/api/stock/000001/price?days=10
```

### Test Go webapp
```bash
curl http://localhost:8080/api/stock/000001/price
```

## 🚨 Troubleshooting

### Common Issues

1. **akshare not found**:
   ```bash
   pip install akshare
   ```

2. **Port already in use**:
   ```bash
   # Find and kill process using port 5000
   lsof -i :5000
   kill -9 [PID]
   ```

3. **Data not loading**:
   - Check if akshare service is running
   - Verify stock codes are valid (6 digits)
   - Check network connectivity

### Fallback Behavior

If the akshare service is not available, the Go webapp will automatically use mock data, ensuring the application remains functional.

## 🐳 Docker Setup

### Docker Compose (Optional)

Create `docker-compose.yml`:

```yaml
version: '3.8'
services:
  akshare-service:
    build: .
    ports:
      - "5000:5000"
    environment:
      - PYTHONUNBUFFERED=1

  go-webapp:
    build: ./go-webapp
    ports:
      - "8080:8080"
    depends_on:
      - akshare-service
    environment:
      - AKSERVICE_URL=http://akshare-service:5000
```

## 📊 Data Quality

The akshare integration provides:

- **Real-time stock prices** (with 15-minute delay for free tier)
- **Historical data** up to 20 years
- **Comprehensive financial metrics**
- **News sentiment analysis**
- **Market indicators**

## 🔗 Additional Resources

- [Akshare Documentation](https://akshare.akfamily.xyz/)
- [Flask Documentation](https://flask.palletsprojects.com/)
- [Go HTTP Client](https://pkg.go.dev/net/http)

## 📝 Development Notes

For development, you can:

1. **Use mock data** for faster development
2. **Start akshare service** for real data testing
3. **Implement caching** for better performance
4. **Add rate limiting** for production use

The architecture is designed to be flexible - you can easily switch between mock data and real akshare data without changing the Go application code.