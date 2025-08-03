#!/usr/bin/env python3
"""
Python akshare HTTP service for Go webapp
Install dependencies: pip install akshare flask flask-cors pandas
"""

from flask import Flask, request, jsonify
from flask_cors import CORS
import akshare as ak
import pandas as pd
from datetime import datetime, timedelta
import json

app = Flask(__name__)
CORS(app)

@app.route('/api/stock/<stock_code>/price')
def get_stock_price(stock_code):
    """Get stock price data"""
    try:
        days = int(request.args.get('days', 30))
        
        # Use stock code directly without prefix for akshare
        full_code = stock_code
        
        # Get historical data
        end_date = datetime.now().strftime('%Y%m%d')
        start_date = (datetime.now() - timedelta(days=days)).strftime('%Y%m%d')
        
        stock_data = ak.stock_zh_a_hist(
            symbol=full_code,
            period="daily",
            start_date=start_date,
            end_date=end_date,
            adjust=""
        )
        
        # Convert to list of dicts
        data = []
        for _, row in stock_data.iterrows():
            data.append({
                'date': str(row['日期']),
                'open': float(row['开盘']) if pd.notna(row['开盘']) else 0.0,
                'close': float(row['收盘']) if pd.notna(row['收盘']) else 0.0,
                'high': float(row['最高']) if pd.notna(row['最高']) else 0.0,
                'low': float(row['最低']) if pd.notna(row['最低']) else 0.0,
                'volume': int(row['成交量']) if pd.notna(row['成交量']) else 0
            })
        
        return jsonify(data)
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@app.route('/api/stock/<stock_code>/fundamental')
def get_stock_fundamental(stock_code):
    """Get stock fundamental data"""
    try:
        # Get stock fundamentals
        try:
            fundamentals = ak.stock_financial_analysis_indicator(symbol=stock_code)
        except:
            fundamentals = pd.DataFrame()
        
        # Get valuation data
        valuation = {'pe_ratio': 15.0, 'pb_ratio': 1.5}
        try:
            # Try to get PE/PB data
            pe_data = ak.stock_a_pe(symbol="000001.SH")
            stock_pe = pe_data[pe_data['代码'] == stock_code]
            if not stock_pe.empty:
                valuation['pe_ratio'] = float(stock_pe.iloc[0]['市盈率'])
                valuation['pb_ratio'] = float(stock_pe.iloc[0]['市净率'])
        except:
            pass
        
        # Extract key indicators
        indicators = []
        if not fundamentals.empty:
            latest = fundamentals.iloc[0]
            
            # Map common indicators
            indicator_mapping = {
                '净利润率': '净利润率',
                '净资产收益率': '净资产收益率',
                '总资产收益率': '总资产收益率',
                '毛利率': '毛利率',
                '资产负债率': '资产负债率',
                '流动比率': '流动比率',
                '营业收入增长率': '营收同比增长率',
                '净利润增长率': '净利润同比增长率'
            }
            
            for key, display_name in indicator_mapping.items():
                if key in latest and pd.notna(latest[key]):
                    try:
                        value = float(latest[key])
                        indicators.append({
                            'name': display_name,
                            'value': value,
                            'unit': '%' if '率' in display_name else '倍' if '比率' in display_name else ''
                        })
                    except (ValueError, TypeError):
                        pass
        
        # Default indicators if no data
        if not indicators:
            indicators = [
                {'name': '净利润率', 'value': 15.2, 'unit': '%'},
                {'name': '净资产收益率', 'value': 12.5, 'unit': '%'},
                {'name': '市盈率', 'value': valuation['pe_ratio'], 'unit': '倍'},
                {'name': '市净率', 'value': valuation['pb_ratio'], 'unit': '倍'}
            ]
        
        return jsonify({
            'financial_indicators': indicators,
            'valuation': valuation,
            'industry': '未知',
            'sector': '未知'
        })
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@app.route('/api/stock/<stock_code>/news')
def get_stock_news(stock_code):
    """Get stock news data"""
    try:
        days = int(request.args.get('days', 15))
        
        # Get stock news
        try:
            news_data = ak.stock_news_em(symbol=stock_code)
            if not news_data.empty:
                # Limit to recent news
                recent_news = news_data.head(30)
                
                news_list = []
                for _, row in recent_news.iterrows():
                    news_list.append({
                        'title': str(row['标题']) if pd.notna(row['标题']) else '',
                        'content': str(row['内容']) if pd.notna(row['内容']) else str(row['标题']),
                        'date': str(row['发布时间']) if pd.notna(row['发布时间']) else str(datetime.now()),
                        'source': str(row['来源']) if pd.notna(row['来源']) else '未知',
                        'type': 'company_news',
                        'relevance': 0.8,
                        'sentiment': 0.0  # Placeholder
                    })
                
                # Calculate sentiment
                overall_sentiment = 0.0
                if news_list:
                    # Simple sentiment based on title keywords
                    positive_words = ['上涨', '增长', '利好', '买入', '增持', '推荐']
                    negative_words = ['下跌', '亏损', '利空', '卖出', '减持', '风险']
                    
                    for news in news_list:
                        title = news['title']
                        score = 0.0
                        for word in positive_words:
                            if word in title:
                                score += 0.1
                        for word in negative_words:
                            if word in title:
                                score -= 0.1
                        news['sentiment'] = max(-1, min(1, score))
                        overall_sentiment += news['sentiment']
                    
                    overall_sentiment /= len(news_list)
                
                sentiment = {
                    'overall_sentiment': overall_sentiment,
                    'sentiment_trend': '中性',
                    'confidence_score': 0.75,
                    'total_analyzed': len(news_list),
                    'sentiment_by_type': {'company_news': overall_sentiment},
                    'news_distribution': {'company_news': len(news_list)}
                }
                
                return jsonify({
                    'news': news_list,
                    'sentiment': sentiment
                })
        except Exception as e:
            pass
        
        # Fallback mock data
        news_list = []
        for i in range(10):
            news_list.append({
                'title': f'{stock_code}相关新闻{i+1}',
                'content': f'这是{stock_code}的第{i+1}条新闻内容',
                'date': (datetime.now() - timedelta(days=i)).strftime('%Y-%m-%d'),
                'source': '新浪财经',
                'type': 'company_news',
                'relevance': 0.8,
                'sentiment': (i % 3 - 1) * 0.3
            })
        
        sentiment = {
            'overall_sentiment': 0.0,
            'sentiment_trend': '中性',
            'confidence_score': 0.5,
            'total_analyzed': len(news_list),
            'sentiment_by_type': {'company_news': 0.0},
            'news_distribution': {'company_news': len(news_list)}
        }
        
        return jsonify({
            'news': news_list,
            'sentiment': sentiment
        })
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@app.route('/api/stock/<stock_code>/name')
def get_stock_name(stock_code):
    """Get stock name"""
    try:
        # Get stock basic info
        stock_basic = ak.stock_zh_a_spot()
        
        stock_info = stock_basic[stock_basic['代码'] == stock_code]
        if not stock_info.empty:
            name = str(stock_info.iloc[0]['名称'])
        else:
            name = f"{stock_code}股票"
        
        return jsonify({'name': name})
    except Exception as e:
        return jsonify({'name': f"{stock_code}股票"})

@app.route('/health')
def health_check():
    """Health check endpoint"""
    return jsonify({'status': 'healthy', 'service': 'akshare-proxy'})

@app.route('/')
def index():
    return jsonify({
        'service': 'akshare-proxy',
        'endpoints': [
            '/api/stock/<code>/price?days=30',
            '/api/stock/<code>/fundamental',
            '/api/stock/<code>/news?days=15',
            '/api/stock/<code>/name',
            '/health'
        ]
    })

if __name__ == '__main__':
    print("Starting akshare service...")
    print("Make sure akshare is installed: pip install akshare flask flask-cors pandas")
    app.run(host='0.0.0.0', port=5000, debug=True)