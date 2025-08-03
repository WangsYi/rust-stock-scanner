# Database initialization script
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Create table for saved analyses
CREATE TABLE IF NOT EXISTS saved_analyses (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    stock_code VARCHAR(20) NOT NULL,
    stock_name VARCHAR(100) NOT NULL,
    analysis_date TIMESTAMP WITH TIME ZONE NOT NULL,
    price_info JSONB NOT NULL,
    technical JSONB NOT NULL,
    fundamental JSONB NOT NULL,
    sentiment JSONB NOT NULL,
    scores JSONB NOT NULL,
    recommendation VARCHAR(50) NOT NULL,
    ai_analysis TEXT,
    data_quality JSONB NOT NULL,
    ai_provider VARCHAR(50),
    ai_model VARCHAR(50),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create table for saved configurations
CREATE TABLE IF NOT EXISTS saved_configurations (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    config_type VARCHAR(50) NOT NULL,
    config_name VARCHAR(100) NOT NULL,
    config_data JSONB NOT NULL,
    is_active BOOLEAN DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for better performance
CREATE INDEX IF NOT EXISTS idx_saved_analyses_stock_code ON saved_analyses(stock_code);
CREATE INDEX IF NOT EXISTS idx_saved_analyses_analysis_date ON saved_analyses(analysis_date);
CREATE INDEX IF NOT EXISTS idx_saved_analyses_created_at ON saved_analyses(created_at);
CREATE INDEX IF NOT EXISTS idx_saved_configurations_type ON saved_configurations(config_type);
CREATE INDEX IF NOT EXISTS idx_saved_configurations_active ON saved_configurations(is_active);

-- Create function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Create trigger for updated_at
DROP TRIGGER IF EXISTS update_saved_configurations_updated_at ON saved_configurations;
CREATE TRIGGER update_saved_configurations_updated_at
    BEFORE UPDATE ON saved_configurations
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Insert default configuration
INSERT INTO saved_configurations (config_type, config_name, config_data, is_active) VALUES
('ai', 'default', '{"provider": "openai", "model": "gpt-4", "enabled": true, "timeout_seconds": 30}', true)
ON CONFLICT DO NOTHING;