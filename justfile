setup-env:
    sudo docker run -d --rm -it \
    -p 127.0.0.1:4566:4566 \
    -p 127.0.0.1:4510-4559:4510-4559 \
    -v /var/run/docker.sock:/var/run/docker.sock \
    localstack/localstack

    podman run -d --rm \
        -e 'ACCEPT_EULA=1' -e 'MSSQL_SA_PASSWORD=Seedy2025' \
        -p 1433:1433 \
        --name azuresqledge \
        mcr.microsoft.com/azure-sql-edge

