#!/usr/bin/env nu

print "Checking Kubernetes tools..."

let tools = ["kubectl" "kubectx" "kubens" "k9s"]
let mise_managed = ["kubectx" "kubens"]

for tool in $tools {
    if (which $tool | is-not-empty) {
        print $"  ($tool) found"
        if ($tool in $mise_managed) {
            print "   (managed by mise)"
        }
    } else {
        print $"  ($tool) not found"
        if ($tool in $mise_managed) {
            print "   Run 'mise install' to install"
        }
    }
}
