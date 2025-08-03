#!/bin/bash

# Setup script for akshare HTTP service
# This script installs and starts the Python akshare service

echo "Setting up akshare service for Go webapp..."

# Check if Python is installed
if ! command -v python3 &> /dev/null; then
    echo "Python3 is not installed. Please install Python3 first."
    exit 1
fi

# Check if pip is installed
if ! command -v pip3 &> /dev/null; then
    echo "pip3 is not installed. Please install pip3 first."
    exit 1
fi

# Install required packages
echo "Installing akshare and dependencies..."
pip3 install akshare flask flask-cors pandas

# Start the akshare service
echo "Starting akshare service on port 5000..."
cd "$(dirname "$0")"
python3 akshare_service.py &

echo "Akshare service started successfully!"
echo "Service URL: http://localhost:5000"
echo "The Go webapp will automatically use real data from this service."
echo "To stop the service, use: pkill -f akshare_service.py"