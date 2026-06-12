#!/usr/bin/env nu

print "Building Kina Kubernetes node image (K8s 1.36.1, containerd 2.3.1, runc 1.4.2, CNI 1.9.1, debian:13-slim)..."
let image_tag = "kina/node:v1.36.1"

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
# container image list output has NAME and TAG as separate columns, not "name:tag"
# Split image_tag into name and tag parts for matching
let tag_parts = ($image_tag | split row ":")
let img_name = ($tag_parts | first)
let img_version = ($tag_parts | last)
if (($images.stdout | str contains $img_name) and ($images.stdout | str contains $img_version)) {
    print $"Successfully built image: ($image_tag)"
} else {
    print "Failed to build image"
    exit 1
}

print ""
print $"Build complete! Use with: mise run kina -- create my-cluster --image ($image_tag)"
