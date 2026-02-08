#!/usr/bin/env nu

let image_tag = "kina/node:v1.31.0"
print $"Testing node image: ($image_tag)"

let images = (do { ^container image list } | complete)
if not ($images.stdout | str contains $image_tag) {
    print $"Image ($image_tag) not found. Run 'mise run image:build' first."
    exit 1
}

print "Image exists"
print "Creating test container..."

let timestamp = (date now | format date "%s")
let test_container = $"kina-node-test-($timestamp)"

let result = (do {
    ^container run --name $test_container --rm -it $image_tag /bin/bash -c "
        echo 'Testing container functionality...'
        echo 'Checking systemd...'
        systemctl --version
        echo 'Checking containerd...'
        containerd --version
        echo 'Checking Kubernetes tools...'
        kubeadm version --output=short
        kubelet --version
        kubectl version --client=true --output=yaml
        echo 'All checks passed!'
    "
} | complete)

print $result.stdout
if $result.exit_code != 0 {
    print "Container test failed"
    exit $result.exit_code
}

print "Node image test completed successfully"
