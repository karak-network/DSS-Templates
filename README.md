# Karak DSS Templates

These are example DSS implementations which can be used as templates to get started with building on Karak.

These templates emulate how a real DSS would function using their respective Docker Compose files with all the actors in separate containers.

## Prerequisites
- Docker Engine installed and running on your machine - [docs.docker.com/engine/install](https://docs.docker.com/engine/install/)
- Docker Compose installed - [docs.docker.com/compose/install](https://docs.docker.com/compose/install/)
- Availability of ports 8080, 8081, 8454, 3000 (You can change the ports in `docker-compose.yaml` if needed)

## Running the DSS Emulation

```bash
docker-compose up --build
