pipeline {
  agent {
    node {
      label 'bach_build'
    }

  }
  stages {
    stage('lint') {
      steps {
        sh '''
source /root/.cargo/env; cargo clippy'''
      }
    }

    stage('test') {
      steps {
        sh 'source /root/.cargo/env; cargo test'
      }
    }

    stage('build') {
      steps {
        sh 'source /root/.cargo/env; cargo build --release'
      }
    }

  }
}