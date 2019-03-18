/*
 * Copyright 2018 astonbitecode
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
package org.astonbitecode.j4rs.api.deploy;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.net.MalformedURLException;
import java.net.URL;
import java.nio.channels.Channels;
import java.nio.channels.ReadableByteChannel;

public class SimpleMavenDeployer {
    private static final String MAVEN_CENTRAL = "https://repo.maven.apache.org/maven2";
    private final String M2_CACHE = System.getProperty("user.home") + File.separator + ".m2" + File.separator + "repository";

    private final String repoBase;
    private final boolean checkLocalCache;
    private final String deployTarget;

    public SimpleMavenDeployer() {
        this(MAVEN_CENTRAL, true, ".");
    }

    public SimpleMavenDeployer(String deployTarget) {
        this(MAVEN_CENTRAL, true, deployTarget);
    }

    public SimpleMavenDeployer(String repoBase, boolean checkLocalCache, String deployTarget) {
        this.repoBase = repoBase;
        this.checkLocalCache = checkLocalCache;
        this.deployTarget = deployTarget;
        new File(deployTarget).mkdirs();
    }

    public void deploy(String groupId, String artifactId, String version, String qualifier) throws MalformedURLException, IOException {
        String jarName = generateArtifactName(artifactId, version, qualifier);
        String urlString = generateUrlTagret(groupId, artifactId, version, jarName);
        boolean searchRemoteRepo = true;

        if (!artifactExists(groupId, artifactId, version, qualifier)) {
            if (checkLocalCache) {
                try {
                    deployFromLocalCache(groupId, artifactId, version, qualifier);
                    searchRemoteRepo = false;
                } catch (Exception error) {
                    /* ignore */
                }
            }
            if (searchRemoteRepo) {
                ReadableByteChannel readableByteChannel = Channels.newChannel(new URL(urlString).openStream());
                FileOutputStream fileOutputStream = new FileOutputStream(deployTarget + File.separator + jarName);
                fileOutputStream.getChannel().transferFrom(readableByteChannel, 0, Long.MAX_VALUE);
            }
        }
    }

    boolean artifactExists(String groupId, String artifactId, String version, String qualifier) {
        String jarName = generateArtifactName(artifactId, version, qualifier);
        String pathString = generatePathTagret(groupId, artifactId, version, jarName);
        return new File(pathString).exists();
    }

    void deployFromLocalCache(String groupId, String artifactId, String version, String qualifier) throws MalformedURLException, IOException {
        String jarName = generateArtifactName(artifactId, version, qualifier);
        String pathString = generatePathTagret(groupId, artifactId, version, jarName);

        ReadableByteChannel readableByteChannel = Channels.newChannel(new File(pathString).toURI().toURL().openStream());
        FileOutputStream fileOutputStream = new FileOutputStream(deployTarget + File.separator + jarName);
        fileOutputStream.getChannel().transferFrom(readableByteChannel, 0, Long.MAX_VALUE);
    }

    String generateArtifactName(String artifactId, String version, String qualifier) {
        StringBuilder jarName = new StringBuilder(String.format("%s-%s", artifactId, version));
        if (qualifier != null && !qualifier.isEmpty()) {
            jarName.append("-").append(qualifier);
        }
        jarName.append(".jar");
        return jarName.toString();
    }

    String generateUrlTagret(String groupId, String artifactId, String version, String jarName) {
        return String.format("%s/%s/%s/%s/%s",
                repoBase,
                groupId.replace(".", "/"),
                artifactId,
                version,
                jarName);
    }

    String generatePathTagret(String groupId, String artifactId, String version, String jarName) {
        return String.format("%s%s%s%s%s%s%s%s%s",
                M2_CACHE,
                File.separator,
                groupId.replace(".", File.separator),
                File.separator,
                artifactId,
                File.separator,
                version,
                File.separator,
                jarName);
    }

    public String getRepoBase() {
        return repoBase;
    }

}
