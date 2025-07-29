# Deployment Guide

## 🐳 Docker Deployment

### Local Development with Docker Compose

```bash
# Build and run with Redis
docker-compose up --build

# Run in background
docker-compose up -d

# View logs
docker-compose logs -f

# Stop services
docker-compose down
```

### Production Docker Build

```bash
# Build the image
docker build -t task-manager .

# Run with external Redis
docker run -p 3000:3000 \
  -e REDIS_URL=redis://your-redis-host:6379 \
  task-manager
```

## 🚀 CI/CD Pipeline

The GitHub Action automatically:

1. **Tests**: Runs Rust tests with Redis service
2. **Quality**: Checks formatting and linting
3. **Build**: Creates multi-platform Docker image
4. **Push**: Publishes to GitHub Container Registry
5. **Deploy**: Placeholder for deployment integration

### Container Registry

Images are published to: `ghcr.io/USERNAME/REPO:latest`

### Environment Variables

- `REDIS_URL`: Redis connection string (default: `redis://127.0.0.1:6379`)
- `RUST_LOG`: Log level (default: `info`)

## ☁️ Cloud Deployment Options

### AWS
```bash
# ECS with Fargate
aws ecs create-service --service-name task-manager \
  --task-definition task-manager:1 \
  --cluster production

# EKS with Kubernetes
kubectl apply -f k8s/
```

### Google Cloud
```bash
# Cloud Run
gcloud run deploy task-manager \
  --image ghcr.io/username/repo:latest \
  --platform managed
```

### Azure
```bash
# Container Instances
az container create --resource-group myResourceGroup \
  --name task-manager \
  --image ghcr.io/username/repo:latest
```

## 🔧 Infrastructure as Code

### Kubernetes Example

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: task-manager
spec:
  replicas: 3
  selector:
    matchLabels:
      app: task-manager
  template:
    metadata:
      labels:
        app: task-manager
    spec:
      containers:
      - name: app
        image: ghcr.io/username/repo:latest
        ports:
        - containerPort: 3000
        env:
        - name: REDIS_URL
          value: "redis://redis-service:6379"
---
apiVersion: v1
kind: Service
metadata:
  name: task-manager-service
spec:
  selector:
    app: task-manager
  ports:
  - port: 80
    targetPort: 3000
  type: LoadBalancer
```

## 🔒 Production Considerations

1. **Security**: Use TLS/SSL certificates
2. **Monitoring**: Add health checks and metrics
3. **Scaling**: Configure horizontal pod autoscaling
4. **Backup**: Implement Redis persistence and backup
5. **Secrets**: Use secure secret management
6. **Networking**: Configure proper firewall rules