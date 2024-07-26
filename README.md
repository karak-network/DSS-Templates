# Karak DSS Templates

These are example DSS implementations which can be used as templates to get started with building on Karak.

These templates emulate how a real DSS would function using their respective docker compose files with all the actors in separate containers.

## Prerequisites
- docker engine installed and running on your machine - https://docs.docker.com/engine/install/
- docker compose installed - https://docs.docker.com/compose/install/
- Availability of ports 8080, 8081, 8454, 3000 (You can change the ports in docker-compose.yaml if needed)

### Running the DSS Emulation

`docker-compose up --build`

