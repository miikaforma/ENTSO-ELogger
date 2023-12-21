-- Description: Create materialized views for the database

-- Create a materialized view for the daily averages in day-ahead prices
CREATE MATERIALIZED VIEW average_kwh_price_day_by_day
    with (timescaledb.continuous) as
SELECT time_bucket('1 day', time, 'Europe/Helsinki') AS date,
    AVG(price / 10) AS avg_price,
    AVG(price / 10 * (tax_percentage / 100 + 1)) AS avg_price_with_tax
FROM day_ahead_prices
WHERE in_domain = '10YFI-1--------U' AND out_domain = '10YFI-1--------U'
GROUP BY date
ORDER BY date;

-- To drop the view for the daily averages in day-ahead prices, run:
-- DROP MATERIALIZED VIEW average_kwh_price_day_by_day;

-- Create a materialized view for the monthly averages in day-ahead prices
CREATE MATERIALIZED VIEW average_kwh_price_month_by_month
    WITH (timescaledb.continuous) AS
SELECT 
    time_bucket('1 month', time, 'Europe/Helsinki') AS date,
    AVG(price / 10) AS avg_price,
    AVG(price / 10 * (tax_percentage / 100 + 1)) AS avg_price_with_tax
FROM 
    day_ahead_prices
WHERE 
    in_domain = '10YFI-1--------U' AND out_domain = '10YFI-1--------U'
GROUP BY 
    date
ORDER BY 
    date;

-- To drop the view for the monthly averages in day-ahead prices, run:
-- DROP MATERIALIZED VIEW average_kwh_price_month_by_month;

-- Create a materialized view for the yearly averages in day-ahead prices
CREATE MATERIALIZED VIEW average_kwh_price_year_by_year
    WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 year', time, 'Europe/Helsinki') AS date,
    AVG(price / 10) AS avg_price,
    AVG(price / 10 * (tax_percentage / 100 + 1)) AS avg_price_with_tax
FROM
    day_ahead_prices
WHERE
    in_domain = '10YFI-1--------U' AND out_domain = '10YFI-1--------U'
GROUP BY    
    date
ORDER BY
    date;

-- To drop the view for the yearly averages in day-ahead prices, run:
-- DROP MATERIALIZED VIEW average_kwh_price_year_by_year;

-- Create a continuous aggregate policy for the view
SELECT add_continuous_aggregate_policy('average_kwh_price_day_by_day',
   start_offset => NULL,
   end_offset => NULL,
   schedule_interval => INTERVAL '1 hour');