setup-env:
    sudo docker run -d --rm -it \
    -p 127.0.0.1:4566:4566 \
    -p 127.0.0.1:4510-4559:4510-4559 \
    -v /var/run/docker.sock:/var/run/docker.sock \
    localstack/localstack

    # docker run -d --rm -e UNFTP_LOG_LEVEL=info -e UNFTP_PASSIVE_PORTS=50000-50005 -p 2121:2121 -p 50000-50020:50000-50020 -p 8080:8080 -e UNFTP_AUTH_TYPE=anonymous -ti bolcom/unftp:v0.15.1-alpine

    podman run -d --rm \
        -e 'ACCEPT_EULA=1' -e 'MSSQL_SA_PASSWORD=Seedy2025' \
        -p 1433:1433 \
        --name azuresqledge \
        mcr.microsoft.com/azure-sql-edge

    podman run -d --rm \
        --name some-postgres \
        -e POSTGRES_PASSWORD=mysecretpassword \
        -p 5432:5432 \
        postgres

teardown:
    sudo docker rm -f $(docker ps -aq)
    podman rm --force --all
