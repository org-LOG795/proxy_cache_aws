FROM rust:1-bullseye

# Import application code in /app directory
RUN mkdir /app
WORKDIR /app
COPY . .

# Read the value from the image_version file and assign it to an environment variable
RUN export IMAGE_VERSION=$(cat image_version)
RUN rm -f image_version

#Build the application
RUN cargo build --release
RUN rm -rf src

#Execute the server on startup
CMD ["target/release/proxy_cache_aws"]