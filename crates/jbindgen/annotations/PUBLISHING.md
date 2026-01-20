# JBindgen Annotations - Publishing to Maven Central

This document describes how to publish the `jbindgen-annotations` library to Maven Central.

## Prerequisites

1. **Maven Account**: Create an account at [Sonatype JIRA](https://issues.sonatype.org/)
2. **GPG Key**: Generate a GPG key for signing artifacts
3. **Maven Settings**: Configure `~/.m2/settings.xml` with credentials

## Setup GPG Key

```bash
# Generate key
gpg --gen-key

# List keys
gpg --list-keys

# Upload to keyserver
gpg --keyserver keyserver.ubuntu.com --send-keys <KEY_ID>
```

## Configure Maven Settings

Add to `~/.m2/settings.xml`:

```xml
<settings>
  <servers>
    <server>
      <id>ossrh</id>
      <username>your-jira-username</username>
      <password>your-jira-password</password>
    </server>
  </servers>
  <profiles>
    <profile>
      <id>ossrh</id>
      <activation>
        <activeByDefault>true</activeByDefault>
      </activation>
      <properties>
        <gpg.executable>gpg</gpg.executable>
        <gpg.passphrase>your-gpg-passphrase</gpg.passphrase>
      </properties>
    </profile>
  </profiles>
</settings>
```

## Publishing Process

### 1. Update Version

Edit `pom.xml` and update the version number.

### 2. Build and Deploy Snapshot

```bash
mvn clean deploy
```

This deploys to the snapshot repository at:
https://oss.sonatype.org/content/repositories/snapshots/

### 3. Release to Maven Central

```bash
# Deploy and release
mvn clean deploy -P release

# Or deploy without auto-release
mvn clean deploy -P release -Dautorelease=false
```

### 4. Verify Release

After release, the artifact will be available at:
- Maven Central: https://repo1.maven.org/maven2/io/github/jni-rs/jbindgen-annotations/
- Search: https://search.maven.org/artifact/io.github.jni-rs/jbindgen-annotations

## Release Profile

Add this profile to `pom.xml` for releases:

```xml
<profiles>
  <profile>
    <id>release</id>
    <build>
      <plugins>
        <plugin>
          <groupId>org.apache.maven.plugins</groupId>
          <artifactId>maven-gpg-plugin</artifactId>
          <version>3.1.0</version>
          <executions>
            <execution>
              <id>sign-artifacts</id>
              <phase>verify</phase>
              <goals>
                <goal>sign</goal>
              </goals>
            </execution>
          </executions>
        </plugin>
        <plugin>
          <groupId>org.sonatype.plugins</groupId>
          <artifactId>nexus-staging-maven-plugin</artifactId>
          <version>1.6.13</version>
          <extensions>true</extensions>
          <configuration>
            <serverId>ossrh</serverId>
            <nexusUrl>https://oss.sonatype.org/</nexusUrl>
            <autoReleaseAfterClose>true</autoReleaseAfterClose>
          </configuration>
        </plugin>
      </plugins>
    </build>
  </profile>
</profiles>
```

## Useful Links

- [OSSRH Guide](https://central.sonatype.org/publish/publish-guide/)
- [Maven Central](https://search.maven.org/)
- [Sonatype JIRA](https://issues.sonatype.org/)
