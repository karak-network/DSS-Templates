FROM node:22.5-slim

# Create app directory
WORKDIR /app

# Copy package.json and yarn.lock
COPY package.json ./

# Install app dependencies
RUN yarn

# Bundle app source
COPY . .

# Expose port 8080
EXPOSE 8080

# Start the app
CMD yarn dev
