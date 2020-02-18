G_giturl = ""
G_gitcred = 'TonJenSSH'
G_docker_creds = "TonJenDockerHub"
G_image_base = "rust:1.40"
G_image_target = ""
G_docker_image = null
G_build = "none"
G_test = "none"

pipeline {
    options {
        buildDiscarder logRotator(artifactDaysToKeepStr: '', artifactNumToKeepStr: '', daysToKeepStr: '', numToKeepStr: '1')
        disableConcurrentBuilds()
        parallelsAlwaysFailFast()
    }
    agent {
        node {
            label 'master'
        }
    }
    parameters {
        string(
            name:'dockerImage_ton_labs_types',
            defaultValue: 'tonlabs/ton-labs-types:latest',
            description: 'Existing ton-labs-types image name'
        )
        string(
            name:'dockerImage_ton_labs_block',
            defaultValue: 'tonlabs/ton-labs-block:latest',
            description: 'Existing ton-labs-block image name'
        )
        string(
            name:'dockerImage_ton_labs_vm',
            defaultValue: 'tonlabs/ton-labs-vm:latest',
            description: 'Existing ton-labs-vm image name'
        )
        string(
            name:'dockerImage_ton_labs_abi',
            defaultValue: '',
            description: 'Expected ton-labs-abi image name'
        )
        string(
            name:'tvm_linker_branch',
            defaultValue: 'master',
            description: 'tvm-linker branch for upstairs test'
        )
        string(
            name:'ton_sdk_branch',
            defaultValue: 'master',
            description: 'ton-sdk branch for upstairs test'
        )
    }
    stages {
        stage('Collect commit data') {
            steps {
                sshagent([G_gitcred]) {
                    script {
                        G_giturl = env.GIT_URL
                        echo "${G_giturl}"
                        C_PROJECT = env.GIT_URL.substring(19, env.GIT_URL.length() - 4)
                        C_COMMITER = sh (script: 'git show -s --format=%cn ${GIT_COMMIT}', returnStdout: true).trim()
                        C_TEXT = sh (script: 'git show -s --format=%s ${GIT_COMMIT}', returnStdout: true).trim()
                        C_AUTHOR = sh (script: 'git show -s --format=%an ${GIT_COMMIT}', returnStdout: true).trim()
                        C_HASH = sh (script: 'git show -s --format=%h ${GIT_COMMIT}', returnStdout: true).trim()
                    
                        DiscordURL = "https://discordapp.com/api/webhooks/496992026932543489/4exQIw18D4U_4T0H76bS3Voui4SyD7yCQzLP9IRQHKpwGRJK1-IFnyZLyYzDmcBKFTJw"
                        string DiscordFooter = "Build duration is ${currentBuild.durationString}"
                        DiscordTitle = "Job ${JOB_NAME} from GitHub ${C_PROJECT}"
                        
                        if (params.dockerImage_ton_labs_abi == '') {
                            G_image_target = "${C_PROJECT}:${GIT_COMMIT}"
                        } else {
                            G_image_target = params.dockerImage_ton_labs_abi
                        }
                        echo "Target image name: ${G_image_target}"

                        def buildCause = currentBuild.getBuildCauses()
                        echo "Build cause: ${buildCause}"
                    }
                }
            }
        }
        stage('Switch to file source') {
            steps {
                script {
                    sh """
(cat Cargo.toml | \
sed 's/ton_types = .*/ton_types = { path = \"\\/tonlabs\\/ton-labs-types\" }/g' | \
sed 's/ton_block = .*/ton_block = { path = \"\\/tonlabs\\/ton-labs-block\" }/g' | \
sed 's/ton_vm = .*/ton_vm = { path = \"\\/tonlabs\\/ton-labs-vm\", default-features = false }/g') > tmp.toml
rm Cargo.toml
mv ./tmp.toml ./Cargo.toml
                    """
                }
            }
        }
        stage('Prepare image') {
            steps {
                echo "Prepare image..."
                script {
                    docker.withRegistry('', G_docker_creds) {
                        args = "--no-cache --label 'git-commit=${GIT_COMMIT}' --force-rm ."
                        G_docker_image = docker.build(
                            G_image_target, 
                            args
                        )
                        echo "Image ${G_docker_image} as ${G_image_target}"
                        G_docker_image.push()
                    }
                }
            }
        }
        stage('Build') {
            steps {
                script {
                    docker.withRegistry('', G_docker_creds) {
                        G_docker_image.withRun() {c -> 
                            docker.image(params.dockerImage_ton_labs_types).withRun() { ton_types_dep ->
                                docker.image(params.dockerImage_ton_labs_block).withRun() { ton_block_dep ->
                                    docker.image(params.dockerImage_ton_labs_vm).withRun() { ton_vm_dep ->
                                        docker.image(G_image_base).inside("--volumes-from ${c.id} --volumes-from ${ton_types_dep.id} --volumes-from ${ton_block_dep.id} --volumes-from ${ton_vm_dep.id}") {
                                            sh """
                                                cd /tonlabs/ton-labs-abi
                                                cargo update
                                                cargo build --release
                                            """
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            post {
                success { script { G_build = "success" } }
                failure { script { G_build = "failure" } }
            }
        }
        stage('Tests') {
            steps {
                script {
                    docker.withRegistry('', G_docker_creds) {
                        G_docker_image.withRun() {c -> 
                            docker.image(params.dockerImage_ton_labs_types).withRun() { ton_types_dep ->
                                docker.image(params.dockerImage_ton_labs_block).withRun() { ton_block_dep ->
                                    docker.image(params.dockerImage_ton_labs_vm).withRun() { ton_vm_dep ->
                                        docker.image(G_image_base).inside("--volumes-from ${c.id} --volumes-from ${ton_types_dep.id} --volumes-from ${ton_block_dep.id} --volumes-from ${ton_vm_dep.id}") {
                                            sh """
                                                cd /tonlabs/ton-labs-abi
                                                cargo test --release --features ci_run
                                            """
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            post {
                success { script { G_test = "success" } }
                failure { script { G_test = "failure" } }
            }
        }
        stage('TON-SDK / TVM_Linker') {
            failFast true
            parallel {
                stage('Build tvm_linker') {
                    steps {
                        script {
                            def params_linker = [
                                [
                                    $class: 'BooleanParameterValue',
                                    name: 'FORCE_PROMOTE_LATEST',
                                    value: false
                                ],
                                [
                                    $class: 'StringParameterValue',
                                    name: 'dockerImage_ton_labs_types',
                                    value: params.dockerImage_ton_labs_types
                                ],
                                [
                                    $class: 'StringParameterValue',
                                    name: 'dockerImage_ton_labs_block',
                                    value: params.dockerImage_ton_labs_block
                                ],
                                [
                                    $class: 'StringParameterValue',
                                    name: 'dockerImage_ton_labs_vm',
                                    value: params.dockerImage_ton_labs_vm
                                ],
                                [
                                    $class: 'StringParameterValue',
                                    name: 'dockerImage_ton_labs_abi',
                                    value: G_image_target
                                ],
                                [
                                    $class: 'StringParameterValue',
                                    name: 'dockerImage_tvm_linker',
                                    value: ''
                                ],
                                [
                                    $class: 'StringParameterValue',
                                    name: 'ton_sdk_branch',
                                    value: params.ton_sdk_branch
                                ]
                            ]
                            build job: "TVM-linker/${params.tvm_linker_branch}", parameters: params_linker
                        }
                    }
                    post {
                        success { script { G_test = "success" } }
                        failure { script { G_test = "failure" } }
                    }
                }
                stage('TON-SDK') {
                    steps {
                        script {
                            def params_ton_sdk = [
                                [
                                    $class: 'StringParameterValue',
                                    name: 'common_version',
                                    value: ''
                                ],
                                [
                                    $class: 'StringParameterValue',
                                    name: 'dockerImage_ton_labs_types',
                                    value: params.dockerImage_ton_labs_types
                                ],
                                [
                                    $class: 'StringParameterValue',
                                    name: 'dockerImage_ton_labs_block',
                                    value: params.dockerImage_ton_labs_block
                                ],
                                [
                                    $class: 'StringParameterValue',
                                    name: 'dockerImage_ton_labs_vm',
                                    value: params.dockerImage_ton_labs_vm
                                ],
                                [
                                    $class: 'StringParameterValue',
                                    name: 'dockerImage_ton_labs_abi',
                                    value: params.dockerImage_ton_labs_abi
                                ],
                                [
                                    $class: 'StringParameterValue',
                                    name: 'dockerImage_ton_executor',
                                    value: 'tonlabs/ton-executor:latest'
                                ],
                                [
                                    $class: 'StringParameterValue',
                                    name: 'ton_sdk_branch',
                                    value: params.ton_sdk_branch
                                ]
                            ]
                            build job: "TON-SDK/${params.ton_sdk_branch}", parameters: params_ton_sdk
                        }
                    }
                }
            }

        }
        stage('Tag as latest') {
            steps {
                script {
                    docker.withRegistry('', G_docker_creds) {
                        G_docker_image.push('latest')
                    }
                }
            }
        }
    }
    post {
        always {
            node('master') {
                script {
                    DiscordDescription = """${C_COMMITER} pushed commit ${C_HASH} by ${C_AUTHOR} with a message '${C_TEXT}'
Build number ${BUILD_NUMBER}
Build: **${G_build}**
Tests: **${G_test}**"""
                    
                    discordSend(
                        title: DiscordTitle, 
                        description: DiscordDescription, 
                        footer: DiscordFooter, 
                        link: RUN_DISPLAY_URL, 
                        successful: currentBuild.resultIsBetterOrEqualTo('SUCCESS'), 
                        webhookURL: DiscordURL
                    )
                    cleanWs notFailBuild: true
                }
            } 
        }
    }
}