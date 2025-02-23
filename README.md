# ENTSO-E Transparency Platform

Day Ahead Prices Logger from the [ENTSO-E Transparency Platform API](https://transparency.entsoe.eu/content/static_content/Static%20content/web%20api/Guide.html)

Requires API Token for usage: https://transparency.entsoe.eu/content/static_content/Static%20content/web%20api/Guide.html#_authentication_and_authorisation

## Docker
https://hub.docker.com/r/miikaforma/entsoe-logger

## Work In Progress

![image](https://user-images.githubusercontent.com/85478566/187778617-faa5fe00-fa8f-4d72-b1d4-d700bddf188d.png)

### Notes

## Usage
Example docker-compose.yml
```
version: "3"

services:
  entsoe_logger:
    restart: unless-stopped
    image: miikaforma/entsoe-logger:latest
    ports:
     - 9092:9092
    volumes:
      - ./configs:/configs:ro
      - ./logs:/logs/:rw
    environment:
      # ENTSO-E API token
      SECURITY_TOKEN: <fillYourTokenHere>
      # ENTSO-E API parameters
      IN_DOMAIN: 10YFI-1--------U
      OUT_DOMAIN: 10YFI-1--------U
      # How often to fetch data (in milliseconds)
      INTERVAL: 3600000 # 3600000 = 1 hour | 10000 = 10 seconds
      # How many days to fetch (start time + interval days)
      INTERVAL_DAYS: 1
      # Start time for fetching data (will only be used if no newer data is found from the database(s))
      START_TIME: '2024-01-01T00:00Z'

      # InfluxDB storage
      INFLUXDB_ENABLED: 'true'
      DATABASE_URL: http://host.docker.internal:8086
      # Connection with authentication
      # DATABASE_URL: http://username:password@host.docker.internal:8086
      # Or after 3.0.1 you can also use
      # INFLUXDB_USERNAME=username
      # INFLUXDB_PASSWORD=password
      DATABASE_NAME: databasename

      # TimeScale DB storage
      TIMESCALEDB_ENABLED: 'true'
      TIMESCALEDB_CONNECTION_STRING: "host=localhost user=myuser password=mysecretpassword dbname=electricity"

      # Operation modes
      ENABLE_REST_API: 'true'
      ENABLE_AUTO_UPDATE: 'true'
```

Also create `logs` and `configs` directories alongside the `docker-compose.yml`.

Then create `production.yaml` file in the `configs` directory with for example the following content:
```
settings:
  - start_time: "2022-11-30T22:00:00"
    end_time: "2023-04-30T20:59:59"
    tax_percentage: 10
  - start_time: "2023-04-30T21:00:00"
    end_time: "2024-08-31T20:59:59"
    tax_percentage: 24
  - start_time: "2024-08-31T21:00:00"
    tax_percentage: 25.5
```