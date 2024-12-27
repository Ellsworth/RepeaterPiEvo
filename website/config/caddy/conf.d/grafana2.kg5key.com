grafana2.kg5key.com:80 {
        reverse_proxy grafana:3000

        encode zstd gzip
}
