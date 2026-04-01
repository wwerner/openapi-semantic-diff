
## API Changes

**39** change(s) detected | Max severity: **breaking**

## Paths

### `/users`

#### GET

- 🔴➖ parameter 'fields' (query) removed — `parameters.fields.query`

- 🔴✏️ parameter 'limit' is now required — `parameters.limit.query`

- 🟡 parameter 'offset' marked as deprecated — `parameters.offset.query`

- 🟢➕ parameter 'sort' (query) added — `parameters.sort.query`

- 🟢➕ property 'avatar_url' added — `responses.200.content.application/json.schema.items.properties.avatar_url`
- 🟢➕ [schema: User] property 'avatar_url' added — `responses.200.content.application/json.schema.items.properties.avatar_url`

- 🔴✏️ maxLength reduced from 200 to 100 — `responses.200.content.application/json.schema.items.properties.email.maxLength`
- 🔴✏️ [schema: User] maxLength reduced from 200 to 100 — `responses.200.content.application/json.schema.items.properties.email.maxLength`

- 🟢✏️ minLength reduced from 5 to 1 — `responses.200.content.application/json.schema.items.properties.email.minLength`
- 🟢✏️ [schema: User] minLength reduced from 5 to 1 — `responses.200.content.application/json.schema.items.properties.email.minLength`

- 🔴➖ property 'name' removed — `responses.200.content.application/json.schema.items.properties.name`
- 🔴➖ [schema: User] property 'name' removed — `responses.200.content.application/json.schema.items.properties.name`

- 🟡 schema marked as deprecated — `responses.200.content.application/json.schema.items.properties.nickname`
- 🟡 [schema: User] schema marked as deprecated — `responses.200.content.application/json.schema.items.properties.nickname`

- 🟢➕ enum value "moderator" added — `responses.200.content.application/json.schema.items.properties.role.enum`
- 🟢➕ [schema: User] enum value "moderator" added — `responses.200.content.application/json.schema.items.properties.role.enum`
- 🔴➖ enum value "viewer" removed — `responses.200.content.application/json.schema.items.properties.role.enum`
- 🔴➖ [schema: User] enum value "viewer" removed — `responses.200.content.application/json.schema.items.properties.role.enum`

- 🔴➖ response '500' removed — `responses.500`

#### POST

- 🟢➕ property 'avatar_url' added — `responses.201.content.application/json.schema.properties.avatar_url`
- 🟢➕ [schema: User] property 'avatar_url' added — `responses.201.content.application/json.schema.properties.avatar_url`

- 🔴✏️ maxLength reduced from 200 to 100 — `responses.201.content.application/json.schema.properties.email.maxLength`
- 🔴✏️ [schema: User] maxLength reduced from 200 to 100 — `responses.201.content.application/json.schema.properties.email.maxLength`

- 🟢✏️ minLength reduced from 5 to 1 — `responses.201.content.application/json.schema.properties.email.minLength`
- 🟢✏️ [schema: User] minLength reduced from 5 to 1 — `responses.201.content.application/json.schema.properties.email.minLength`

- 🔴➖ property 'name' removed — `responses.201.content.application/json.schema.properties.name`
- 🔴➖ [schema: User] property 'name' removed — `responses.201.content.application/json.schema.properties.name`

- 🟡 schema marked as deprecated — `responses.201.content.application/json.schema.properties.nickname`
- 🟡 [schema: User] schema marked as deprecated — `responses.201.content.application/json.schema.properties.nickname`

- 🟢➕ enum value "moderator" added — `responses.201.content.application/json.schema.properties.role.enum`
- 🟢➕ [schema: User] enum value "moderator" added — `responses.201.content.application/json.schema.properties.role.enum`
- 🔴➖ enum value "viewer" removed — `responses.201.content.application/json.schema.properties.role.enum`
- 🔴➖ [schema: User] enum value "viewer" removed — `responses.201.content.application/json.schema.properties.role.enum`

### `/users/{userId}`

#### GET

- 🟢➕ property 'avatar_url' added — `responses.200.content.application/json.schema.properties.avatar_url`
- 🟢➕ [schema: User] property 'avatar_url' added — `responses.200.content.application/json.schema.properties.avatar_url`

- 🔴✏️ maxLength reduced from 200 to 100 — `responses.200.content.application/json.schema.properties.email.maxLength`
- 🔴✏️ [schema: User] maxLength reduced from 200 to 100 — `responses.200.content.application/json.schema.properties.email.maxLength`

- 🟢✏️ minLength reduced from 5 to 1 — `responses.200.content.application/json.schema.properties.email.minLength`
- 🟢✏️ [schema: User] minLength reduced from 5 to 1 — `responses.200.content.application/json.schema.properties.email.minLength`

- 🔴➖ property 'name' removed — `responses.200.content.application/json.schema.properties.name`
- 🔴➖ [schema: User] property 'name' removed — `responses.200.content.application/json.schema.properties.name`

- 🟡 schema marked as deprecated — `responses.200.content.application/json.schema.properties.nickname`
- 🟡 [schema: User] schema marked as deprecated — `responses.200.content.application/json.schema.properties.nickname`

- 🟢➕ enum value "moderator" added — `responses.200.content.application/json.schema.properties.role.enum`
- 🟢➕ [schema: User] enum value "moderator" added — `responses.200.content.application/json.schema.properties.role.enum`
- 🔴➖ enum value "viewer" removed — `responses.200.content.application/json.schema.properties.role.enum`
- 🔴➖ [schema: User] enum value "viewer" removed — `responses.200.content.application/json.schema.properties.role.enum`

#### DELETE

- 🟡 operation marked as deprecated

### `/users/{userId}/avatar`

#### PUT

- 🔴➖ endpoint PUT /users/{userId}/avatar removed

### `/users/{userId}/settings`

#### GET

- 🟢➕ endpoint GET /users/{userId}/settings added

## Metadata

### Info

- 🟢✏️ version changed from '1.0.0' to '2.0.0' — `version`

### Schemas › LegacyProfile

- 🔴➖ schema 'LegacyProfile' removed — `components.schemas.LegacyProfile`

### Schemas › Settings

- 🟢➕ schema 'Settings' added — `components.schemas.Settings`

