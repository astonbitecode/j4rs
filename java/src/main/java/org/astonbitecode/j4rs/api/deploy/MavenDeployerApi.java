/*
 * Copyright 2026 astonbitecode
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

import java.io.IOException;

public interface MavenDeployerApi {

    /**
     * Deploy a Maven artifact 
     * 
     * @param groupId The maven group ID
     * @param artifactId The maven artifact ID
     * @param version The artifact version
     * @param qualifier The artifact qualifier
     * @param artifactType The artifact type (eg. jar, pom etc)
     * @throws IOException
     */
    void deploy(String groupId, String artifactId, String version, String qualifier, String artifactType) throws IOException;

    /**
     * Deploy a Maven artifact 
     * 
     * @param groupId The maven group ID
     * @param artifactId The maven artifact ID
     * @param version The artifact version
     * @param qualifier The artifact qualifier
     * @throws IOException
     */
    void deploy(String groupId, String artifactId, String version, String qualifier) throws IOException;

    /**
     * Returns the maven repository base (eg. the maven central)
     * 
     * @return The maven repository base
     */
    String getRepoBase();

    /**
     * Returns the j4rs deployment target. ie. the location where the rust application will search to find
     * the maven artifacts
     * 
     * @return The deployment target
     */
    String getDeployTarget();
}
