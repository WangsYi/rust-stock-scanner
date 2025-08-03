#!/bin/bash

# Database setup script for Rust Stock Analyzer

echo "🗄️ Setting up database for Rust Stock Analyzer..."

# Default database settings
DB_TYPE="sqlite"
DB_NAME="stock_analyzer.db"
DB_HOST="localhost"
DB_PORT="5432"

# Check if DATABASE_URL is set
if [ -n "$DATABASE_URL" ]; then
    echo "🔧 Using DATABASE_URL from environment: $DATABASE_URL"
    if [[ "$DATABASE_URL" == postgres* ]]; then
        DB_TYPE="postgres"
        DB_NAME=$(echo "$DATABASE_URL" | sed 's/.*\/\([^\/]*\)$/\1/')
        DB_HOST=$(echo "$DATABASE_URL" | sed 's/.*@\([^:]*\):.*/\1/')
        DB_PORT=$(echo "$DATABASE_URL" | sed 's/.*:\([0-9]*\)\/.*/\1/')
    elif [[ "$DATABASE_URL" == sqlite* ]]; then
        DB_TYPE="sqlite"
        DB_NAME=$(echo "$DATABASE_URL" | sed 's/.*:\(.*\)$/\1/')
    fi
else
    echo "🔧 Using default SQLite database"
fi

if [ "$DB_TYPE" = "postgres" ]; then
    echo "📋 Setting up PostgreSQL database..."
    
    # Check if psql is installed
    if ! command -v psql &> /dev/null; then
        echo "❌ PostgreSQL not found. Please install PostgreSQL first."
        echo "   Ubuntu/Debian: sudo apt-get install postgresql postgresql-contrib"
        echo "   macOS: brew install postgresql"
        echo "   Windows: Download from https://www.postgresql.org/download/"
        exit 1
    fi

    # Default PostgreSQL settings
    DB_USER="${DB_USER:-postgres}"

    # Check if database exists
    DB_EXISTS=$(psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -tAc "SELECT 1 FROM pg_database WHERE datname='$DB_NAME'" 2>/dev/null)

    if [ "$DB_EXISTS" = "1" ]; then
        echo "✅ Database '$DB_NAME' already exists"
    else
        echo "📦 Creating database '$DB_NAME'..."
        createdb -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" "$DB_NAME"
        if [ $? -eq 0 ]; then
            echo "✅ Database '$DB_NAME' created successfully"
        else
            echo "❌ Failed to create database '$DB_NAME'"
            exit 1
        fi
    fi

    # Test connection
    echo "🔗 Testing database connection..."
    if psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "SELECT 1;" > /dev/null 2>&1; then
        echo "✅ Database connection successful"
    else
        echo "❌ Database connection failed"
        echo "   Please check your PostgreSQL settings and user permissions"
        exit 1
    fi

    echo ""
    echo "📋 PostgreSQL Configuration:"
    echo "   Database Name: $DB_NAME"
    echo "   Host: $DB_HOST"
    echo "   Port: $DB_PORT"
    echo "   User: $DB_USER"
    echo ""
    echo "🔧 Environment variables for the application:"
    echo "   export DATABASE_URL=postgres://$DB_USER@$DB_HOST:$DB_PORT/$DB_NAME"
    echo "   export DATABASE_MAX_CONNECTIONS=5"
    echo "   export DATABASE_ENABLE_MIGRATIONS=true"
    echo ""
    echo "🚀 You can now run the application with:"
    echo "   DATABASE_URL=postgres://$DB_USER@$DB_HOST:$DB_PORT/$DB_NAME cargo run"

else
    echo "📋 Setting up SQLite database..."
    
    # For SQLite, just create the database file if it doesn't exist
    if [ -f "$DB_NAME" ]; then
        echo "✅ SQLite database '$DB_NAME' already exists"
    else
        echo "📦 Creating SQLite database '$DB_NAME'..."
        touch "$DB_NAME"
        if [ $? -eq 0 ]; then
            echo "✅ SQLite database '$DB_NAME' created successfully"
        else
            echo "❌ Failed to create SQLite database '$DB_NAME'"
            exit 1
        fi
    fi

    # Test SQLite connection (basic file check)
    if [ -f "$DB_NAME" ]; then
        echo "✅ SQLite database file is accessible"
    else
        echo "❌ SQLite database file is not accessible"
        exit 1
    fi

    echo ""
    echo "📋 SQLite Configuration:"
    echo "   Database File: $DB_NAME"
    echo ""
    echo "🔧 Environment variables for the application:"
    echo "   export DATABASE_URL=sqlite:$DB_NAME"
    echo "   export DATABASE_MAX_CONNECTIONS=5"
    echo "   export DATABASE_ENABLE_MIGRATIONS=true"
    echo ""
    echo "🚀 You can now run the application with:"
    echo "   DATABASE_URL=sqlite:$DB_NAME cargo run"
fi

echo ""
echo "🎉 Database setup complete!"