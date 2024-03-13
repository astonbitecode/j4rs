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

import org.w3c.dom.Document;
import org.w3c.dom.Node;
import org.w3c.dom.NodeList;
import org.xml.sax.SAXException;

import javax.xml.parsers.DocumentBuilder;
import javax.xml.parsers.DocumentBuilderFactory;
import javax.xml.parsers.ParserConfigurationException;
import javax.xml.xpath.XPath;
import javax.xml.xpath.XPathConstants;
import javax.xml.xpath.XPathExpressionException;
import javax.xml.xpath.XPathFactory;
import java.io.*;
import java.net.MalformedURLException;
import java.net.URL;
import java.nio.channels.Channels;
import java.nio.channels.ReadableByteChannel;

public class SimpleMavenDeployer {
    private static final String MAVEN_CENTRAL = "https://repo.maven.apache.org/maven2";
    private final String M2_CACHE = System.getProperty("user.home") + File.separator + ".m2" + File.separator
            + "repository";

    private final String repoBase;
    private final boolean checkLocalCache;
    private final String deployTarget;

    public SimpleMavenDeployer() {
        this(MAVEN_CENTRAL, true, ".");
    }

    public SimpleMavenDeployer(String deployTarget) {
        this(MAVEN_CENTRAL, true, deployTarget);
    }

    public SimpleMavenDeployer(String repoBase, String deployTarget) {
        this(repoBase, true, deployTarget);
    }

    public SimpleMavenDeployer(String repoBase, boolean checkLocalCache, String deployTarget) {
        this.repoBase = repoBase;
        this.checkLocalCache = checkLocalCache;
        this.deployTarget = deployTarget;
        new File(deployTarget).mkdirs();
    }

    public void deploy(String groupId, String artifactId, String version, String qualifier) throws IOException {
        String jarName = generateArtifactName(artifactId, version, qualifier);
        boolean searchRemoteRepo = true;

        if (!artifactExists(groupId, artifactId, version, qualifier)) {
            String fullJarDeployPath = deployTarget + File.separator + jarName;
            if (checkLocalCache) {
                try {
                    deployFromLocalCache(groupId, artifactId, version, qualifier);
                    searchRemoteRepo = false;
                } catch (Exception error) {
                    /* ignore */
                }
            }
            if (searchRemoteRepo) {
                String urlString = generateUrlTagret(groupId, artifactId, version, jarName);
                ReadableByteChannel readableByteChannel = Channels.newChannel(new URL(urlString).openStream());
                try (FileOutputStream fileOutputStream = new FileOutputStream(fullJarDeployPath)) {
                    fileOutputStream.getChannel().transferFrom(readableByteChannel, 0, Long.MAX_VALUE);
                }
            }

            DeployUtils.addToClasspath(fullJarDeployPath);
        }
    }

    private boolean artifactExists(String groupId, String artifactId, String version, String qualifier) {
        String jarName = generateArtifactName(artifactId, version, qualifier);
        String pathString = deployTarget + File.separator + jarName;
        return new File(pathString).exists();
    }

    void deployFromLocalCache(String groupId, String artifactId, String version, String qualifier)
            throws MalformedURLException, IOException {
        String jarName = generateArtifactName(artifactId, version, qualifier);
        String pathString = generatePathTagret(M2_CACHE, groupId, artifactId, version, jarName);

        ReadableByteChannel readableByteChannel = Channels
                .newChannel(new File(pathString).toURI().toURL().openStream());
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

    String generateUrlTagret(String groupId, String artifactId, String version, String jarName) throws IOException {
        if (version.endsWith("-SNAPSHOT")) {
            String latestSnapshotJarName = getLatestSnapshotName(groupId, artifactId, version);
            return  String.format("%s/%s/%s/%s/%s", repoBase, groupId.replace(".", "/"), artifactId, version, latestSnapshotJarName);
        } else {
            return String.format("%s/%s/%s/%s/%s", repoBase, groupId.replace(".", "/"), artifactId, version, jarName);
        }
    }

    private String getLatestSnapshotName(String groupId, String artifactId, String version) throws IOException {
        String metadataXmlUrl = String.format("%s/%s/%s/%s/%s", repoBase, groupId.replace(".", "/"), artifactId, version, "maven-metadata.xml");
        ReadableByteChannel readableByteChannel = Channels.newChannel(new URL(metadataXmlUrl).openStream());
        try (InputStream inputStream = Channels.newInputStream(readableByteChannel)) {
            DocumentBuilderFactory builderFactory = DocumentBuilderFactory.newInstance();
            DocumentBuilder builder = builderFactory.newDocumentBuilder();
            Document xmlDocument = builder.parse(inputStream);
            XPath xPath = XPathFactory.newInstance().newXPath();
            String timestamp = xPath.evaluate("/metadata/versioning/snapshot/timestamp", xmlDocument);
            String buildNumber = xPath.evaluate("/metadata/versioning/snapshot/buildNumber", xmlDocument);
            String snapshotVersion = version.replace("SNAPSHOT", (timestamp + "-" + buildNumber));
            return  String.format("%s-%s.jar", artifactId, snapshotVersion);
        } catch (XPathExpressionException | ParserConfigurationException | SAXException e) {
            throw new RuntimeException(e);
        }
    }

    String generatePathTagret(String base, String groupId, String artifactId, String version, String jarName) {
        return String.format("%s%s%s%s%s%s%s%s%s", base, File.separator, groupId.replace(".", File.separator),
                File.separator, artifactId, File.separator, version, File.separator, jarName);
    }

    public String getRepoBase() {
        return repoBase;
    }

}
