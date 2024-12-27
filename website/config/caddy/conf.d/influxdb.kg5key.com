influxdb.kg5key.com:80 {
        reverse_proxy influxdb:8086

        encode zstd gzip
}
