#!/usr/bin/env nu

print "Building Kina Kubernetes node image..."
let image_tag = "kina/node:v1.31.0"

cd kina-cli/images

print "Building with Apple Container..."
let build = (do { ^container build -t $image_tag . } | complete)
print $build.stdout
if $build.exit_code != 0 {
    print $build.stderr
    print "Failed to build image"
    exit 1
}

let images = (do { ^container image list } | complete)
if ($images.stdout | str contains $image_tag) {
    print $"Successfully built image: ($image_tag)"
} else {
    print "Failed to build image"
    exit 1
}

print ""
print $"Build complete! Use with: mise run kina -- create my-cluster --image ($image_tag)"
