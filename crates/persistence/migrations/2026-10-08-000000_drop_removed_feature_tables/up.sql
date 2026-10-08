-- Drop the tables of features that Alefbet removed: Warp Drive and cloud objects, workflows
-- and notebooks stored in the cloud, env var collections, teams and workspaces, the signed-in
-- user, server experiments, and the agent, AI and MCP features. Tables that reference another
-- dropped table are dropped first so foreign keys stay satisfied.

DROP TABLE IF EXISTS agent_tasks;
DROP TABLE IF EXISTS object_permissions;
DROP TABLE IF EXISTS team_members;
DROP TABLE IF EXISTS team_settings;
DROP TABLE IF EXISTS workspace_teams;
DROP TABLE IF EXISTS agent_conversations;
DROP TABLE IF EXISTS object_metadata;
DROP TABLE IF EXISTS teams;
DROP TABLE IF EXISTS workspaces;
DROP TABLE IF EXISTS active_mcp_servers;
DROP TABLE IF EXISTS ai_document_panes;
DROP TABLE IF EXISTS ai_memory_panes;
DROP TABLE IF EXISTS ai_queries;
DROP TABLE IF EXISTS ambient_agent_panes;
DROP TABLE IF EXISTS cloud_objects_refreshes;
DROP TABLE IF EXISTS current_user_information;
DROP TABLE IF EXISTS env_var_collection_panes;
DROP TABLE IF EXISTS folders;
DROP TABLE IF EXISTS generic_string_objects;
DROP TABLE IF EXISTS mcp_environment_variables;
DROP TABLE IF EXISTS mcp_server_installations;
DROP TABLE IF EXISTS mcp_server_panes;
DROP TABLE IF EXISTS notebooks;
DROP TABLE IF EXISTS object_actions;
DROP TABLE IF EXISTS project_rules;
DROP TABLE IF EXISTS server_experiments;
DROP TABLE IF EXISTS user_profiles;
DROP TABLE IF EXISTS users;
DROP TABLE IF EXISTS workflow_panes;
DROP TABLE IF EXISTS workflows;
