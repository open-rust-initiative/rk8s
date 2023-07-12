# Rk8s - A Kubernetes Distribution with Rust

For now, we are woking on a deploy tool helps you set up a working kubernetes cluster in just a few steps.

## Usage

Prerequisite:

[CentOS Stream 9](https://mirror.stream.centos.org/9-stream/BaseOS/x86_64/iso/): the currently tested working distro, since the rust container runtime [youki](https://github.com/containers/youki) has a limit on linux kernel version.

gcc: compiler which rustc depends on.

[cargo](https://rustup.rs/) command available: which means you need to have rust installed.

cfssl: If you do not have one installed, you can run `rk8s install cfssl` after the configuration is generated.

First, clone & build the crate:

```bash
git clone https://github.com/open-rust-initiative/rk8s.git
cd rk8s
cargo build
cp target/debug/rk8s /usr/bin
```

Then you should now have a working `rk8s` ready for deploying.

> The `rk8s` command needs root privilege.

#### Deploying outside the cluster:

1. `rk8s generate config` will generate a folder named `rk8s` under `/root` directory.

2. Change the content in `/root/rk8s/cfg/config.yaml`, specify the machines' IP addresses and their according roles (master or worker) in `instance_hosts` filed, if your deploying machine (the machine running `rk8s`) will be outside of cluster, then `instance_ip` and `instance_name` fields are irrelevant.

3. `ssh-keygen` to generate a key for ssh connection across machines, and `ssh-copy-id -i <path/to/.pub> root@<IP address>` notifies machines to be deployed.

4. `rk8s install cfssl` to install cfssl-related tools for later use.

5. Run `rk8s deploy`.

Then you should have a working cluster, ssh to the master node and run `kubectl get nodes`, you should see the master node is ready.

#### Deploying inside the cluster:

1. Run `rk8s generate config` on every machine to be deployed.

2. Run `rk8s precheck` to turn off *selinux*, *firewall* and so forth.

3. Change the content in `/root/rk8s/cfg/config.yaml`, specify the machines' IP addresses and their according roles (master or worker) in `instance_hosts` filed, `instance_ip` and `instance_name` fields need to be set correctly according to the `IP` and `hostname` of the current machine.

4. Run `rk8s install etcd` on each instance.

5. For masters run:
   
   ```bash
   $ rk8s install kubernetes
   $ rk8s install docker
   ```

6. Run `rk8s init` on master node.

7. Run `rk8s join` on worker node.

After `rk8s` finishes its job, the cluster is now set, the deploying machine is inside the cluster.

