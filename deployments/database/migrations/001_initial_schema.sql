-- Nexa-net Initial Database Schema
-- PostgreSQL migration script

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ============================================================================
-- Identity Tables
-- ============================================================================

-- DID Documents
CREATE TABLE IF NOT EXISTS did_documents (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    did VARCHAR(255) NOT NULL UNIQUE,
    document JSONB NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    CONSTRAINT did_format CHECK (did LIKE 'did:nexa:%')
);

CREATE INDEX idx_did_documents_did ON did_documents(did);
CREATE INDEX idx_did_documents_created ON did_documents(created_at);

-- Verifiable Credentials
CREATE TABLE IF NOT EXISTS credentials (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    credential_id VARCHAR(255) NOT NULL UNIQUE,
    issuer_did VARCHAR(255) NOT NULL,
    subject_did VARCHAR(255) NOT NULL,
    credential_type VARCHAR(255) NOT NULL,
    credential_data JSONB NOT NULL,
    issuance_date TIMESTAMP WITH TIME ZONE NOT NULL,
    expiration_date TIMESTAMP WITH TIME ZONE,
    proof JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    CONSTRAINT issuer_did_fk FOREIGN KEY (issuer_did) REFERENCES did_documents(did) ON DELETE CASCADE
);

CREATE INDEX idx_credentials_issuer ON credentials(issuer_did);
CREATE INDEX idx_credentials_subject ON credentials(subject_did);
CREATE INDEX idx_credentials_type ON credentials(credential_type);
CREATE INDEX idx_credentials_expiration ON credentials(expiration_date);

-- ============================================================================
-- Discovery Tables
-- ============================================================================

-- Capabilities Registry
CREATE TABLE IF NOT EXISTS capabilities (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    did VARCHAR(255) NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    tags JSONB DEFAULT '[]',
    endpoints JSONB DEFAULT '[]',
    cost_model JSONB DEFAULT '{}',
    quality_metrics JSONB DEFAULT '{}',
    rate_limit JSONB DEFAULT '{}',
    embedding_vector VECTOR(384),  -- Requires pgvector extension
    available BOOLEAN DEFAULT TRUE,
    registered_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    CONSTRAINT capability_did_fk FOREIGN KEY (did) REFERENCES did_documents(did) ON DELETE CASCADE
);

CREATE INDEX idx_capabilities_did ON capabilities(did);
CREATE INDEX idx_capabilities_name ON capabilities(name);
CREATE INDEX idx_capabilities_available ON capabilities(available);
CREATE INDEX idx_capabilities_registered ON capabilities(registered_at);
-- GIN index for JSONB tags
CREATE INDEX idx_capabilities_tags ON capabilities USING GIN (tags);

-- ============================================================================
-- Economy Tables
-- ============================================================================

-- Payment Channels
CREATE TABLE IF NOT EXISTS channels (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    channel_id VARCHAR(255) NOT NULL UNIQUE,
    party_a_did VARCHAR(255) NOT NULL,
    party_b_did VARCHAR(255) NOT NULL,
    deposit_a BIGINT NOT NULL DEFAULT 0,
    deposit_b BIGINT NOT NULL DEFAULT 0,
    balance_a BIGINT NOT NULL DEFAULT 0,
    balance_b BIGINT NOT NULL DEFAULT 0,
    state VARCHAR(50) NOT NULL DEFAULT 'open',
    challenge_period_seconds INTEGER DEFAULT 3600,
    dispute_state JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    closed_at TIMESTAMP WITH TIME ZONE,
    
    CONSTRAINT party_a_did_fk FOREIGN KEY (party_a_did) REFERENCES did_documents(did) ON DELETE CASCADE,
    CONSTRAINT party_b_did_fk FOREIGN KEY (party_b_did) REFERENCES did_documents(did) ON DELETE CASCADE,
    CONSTRAINT valid_state CHECK (state IN ('opening', 'open', 'closing', 'closed', 'disputed'))
);

CREATE INDEX idx_channels_id ON channels(channel_id);
CREATE INDEX idx_channels_party_a ON channels(party_a_did);
CREATE INDEX idx_channels_party_b ON channels(party_b_did);
CREATE INDEX idx_channels_state ON channels(state);
CREATE INDEX idx_channels_created ON channels(created_at);

-- Micro Receipts
CREATE TABLE IF NOT EXISTS receipts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    call_id VARCHAR(255) NOT NULL,
    payer_did VARCHAR(255) NOT NULL,
    payee_did VARCHAR(255) NOT NULL,
    amount BIGINT NOT NULL,
    channel_id VARCHAR(255) NOT NULL,
    signature BYTEA,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    CONSTRAINT payer_did_fk FOREIGN KEY (payer_did) REFERENCES did_documents(did) ON DELETE CASCADE,
    CONSTRAINT payee_did_fk FOREIGN KEY (payee_did) REFERENCES did_documents(did) ON DELETE CASCADE,
    CONSTRAINT receipt_channel_fk FOREIGN KEY (channel_id) REFERENCES channels(channel_id) ON DELETE CASCADE
);

CREATE INDEX idx_receipts_call_id ON receipts(call_id);
CREATE INDEX idx_receipts_payer ON receipts(payer_did);
CREATE INDEX idx_receipts_payee ON receipts(payee_did);
CREATE INDEX idx_receipts_channel ON receipts(channel_id);
CREATE INDEX idx_receipts_created ON receipts(created_at);

-- ============================================================================
-- Audit Tables
-- ============================================================================

-- Audit Events
CREATE TABLE IF NOT EXISTS audit_events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    event_type VARCHAR(100) NOT NULL,
    actor_did VARCHAR(255),
    target_did VARCHAR(255),
    channel_id VARCHAR(255),
    call_id VARCHAR(255),
    ip_address VARCHAR(45),
    details JSONB,
    severity VARCHAR(20) DEFAULT 'info',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    CONSTRAINT valid_severity CHECK (severity IN ('info', 'warning', 'error', 'critical'))
);

CREATE INDEX idx_audit_events_type ON audit_events(event_type);
CREATE INDEX idx_audit_events_actor ON audit_events(actor_did);
CREATE INDEX idx_audit_events_created ON audit_events(created_at);
CREATE INDEX idx_audit_events_severity ON audit_events(severity);

-- ============================================================================
-- Node Status Tables
-- ============================================================================

-- Node Status Tracking
CREATE TABLE IF NOT EXISTS node_status (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    did VARCHAR(255) NOT NULL UNIQUE,
    load_factor FLOAT DEFAULT 0.0,
    latency_ms INTEGER DEFAULT 0,
    success_count INTEGER DEFAULT 0,
    failure_count INTEGER DEFAULT 0,
    last_success_at TIMESTAMP WITH TIME ZONE,
    last_failure_at TIMESTAMP WITH TIME ZONE,
    last_heartbeat_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    is_healthy BOOLEAN DEFAULT TRUE,
    
    CONSTRAINT node_did_fk FOREIGN KEY (did) REFERENCES did_documents(did) ON DELETE CASCADE
);

CREATE INDEX idx_node_status_did ON node_status(did);
CREATE INDEX idx_node_status_healthy ON node_status(is_healthy);
CREATE INDEX idx_node_status_heartbeat ON node_status(last_heartbeat_at);

-- ============================================================================
-- Functions and Triggers
-- ============================================================================

-- Update timestamp trigger function
CREATE OR REPLACE FUNCTION update_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply triggers to tables with updated_at
CREATE TRIGGER trigger_did_documents_updated
    BEFORE UPDATE ON did_documents
    FOR EACH ROW EXECUTE FUNCTION update_timestamp();

CREATE TRIGGER trigger_capabilities_updated
    BEFORE UPDATE ON capabilities
    FOR EACH ROW EXECUTE FUNCTION update_timestamp();

CREATE TRIGGER trigger_channels_updated
    BEFORE UPDATE ON channels
    FOR EACH ROW EXECUTE FUNCTION update_timestamp();

-- ============================================================================
-- Views
-- ============================================================================

-- Active capabilities view
CREATE OR REPLACE VIEW active_capabilities AS
SELECT 
    c.did,
    c.name,
    c.description,
    c.tags,
    c.endpoints,
    c.cost_model,
    c.quality_metrics,
    c.available,
    n.load_factor,
    n.latency_ms,
    n.is_healthy
FROM capabilities c
LEFT JOIN node_status n ON c.did = n.did
WHERE c.available = TRUE AND n.is_healthy = TRUE;

-- Channel summary view
CREATE OR REPLACE VIEW channel_summary AS
SELECT 
    channel_id,
    party_a_did,
    party_b_did,
    balance_a + balance_b AS total_balance,
    state,
    created_at,
    EXTRACT(EPOCH FROM (NOW() - created_at))::INTEGER AS age_seconds
FROM channels
WHERE state != 'closed';

-- ============================================================================
-- Initial Data
-- ============================================================================

-- Insert default trust anchor (if needed)
-- This would be replaced with actual trust anchor in production
INSERT INTO did_documents (did, document) VALUES (
    'did:nexa:trust-anchor',
    '{"id": "did:nexa:trust-anchor", "verificationMethod": [], "service": []}'
) ON CONFLICT (did) DO NOTHING;