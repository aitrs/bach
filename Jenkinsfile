pipeline {
  agent {
    node {
      label 'bach_build'
    }

  }
  stages {
    stage('env') {
      steps {
        sh 'source /root/.cargo/env'
      }
    }

    stage('lint') {
      steps {
        sh 'cargo clippy'
      }
    }

    stage('test') {
      steps {
        sh 'cargo test'
      }
    }

    stage('build') {
      steps {
        sh 'cargo build --release'
      }
    }

  }
}