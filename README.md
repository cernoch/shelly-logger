# shelly-logger

Service that reads [Shelly Plug](https://shelly-api-docs.shelly.cloud/gen1/#shelly-plug-plugs) metering
statistics and saves it into [InfluxDB 2](https://docs.influxdata.com/influxdb/v2.6/reference/api/) database.

Intead of using this project, you could hook MQTT stream. But that only gives you _instantaneous_ power
([`shellies/<model>-<deviceid>/relay/0/power`](https://shelly-api-docs.shelly.cloud/gen1/#shelly-plug-plugs-mqtt)).
This project _also_ logs the [_counters_](https://shelly-api-docs.shelly.cloud/gen1/#shelly-plug-plugs-meter-0) value,
which has the by-minute-averages done by the Plug. This should increase the accuracy of the measurements.

## Quickstart

Create a `docker-compose.yml` file:

```yml
version: '3.4'
services:
  thumbsup-cron:
    image: "ghcr.io/cernoch/shelly-logger:latest"
    user: [IDEALLY_A_NON_ROOT_USER_ON_YOUR_PC]
    restart: always
    volumes:
      - "[PATH_TO_CONFIG_FILE]:/etc/shelly-logger:ro"
```

And the just `docker compose up --detach`.

The `[PATH_TO_CONFIG_FILE]` directory must contain the
[`config.json`](app/config.json) file adjusted to your own setup.



## How to build yourself

```
$ git clone https://github.com/cernoch/shelly-logger.git
$ cd shelly-logger
$ docker build -t ghcr.io/cernoch/shelly-logger:latest .
```



## How to release

- Release commit is tagged as `v[MAJOR].[PATCH]`.
- Backwards compatible changes may only bump the `PATCH` version.
- When bumping the `PATCH` version, just tag the commit & push.
- When bumping the `MAJOR` version, also update the `ghcr.io/cernoch/shelly-logger:[MAJOR]` tag in `.github/workflows/main.yml`.