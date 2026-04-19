# RAG Workflow Tutorial

This tutorial demonstrates how to use MPL for a typed Retrieval-Augmented Generation (RAG) workflow.

## Overview

You'll learn how to:
1. Send typed RAG queries (`eval.rag.RAGQuery.v1`)
2. Receive typed responses (`eval.rag.RAGResponse.v1`)
3. Track groundedness and confidence scores
4. Validate source citations

## Prerequisites

- MPL proxy running (`docker compose up -d`)
- Python 3.10+

## The RAG STypes

### RAGQuery.v1
```json
{
  "query": "What are the key features?",
  "context": {
    "maxDocuments": 5,
    "minRelevanceScore": 0.7,
    "sources": ["product-docs"]
  }
}
```

### RAGResponse.v1
```json
{
  "answer": "The key features are...",
  "sources": [
    {"documentId": "doc-001", "relevanceScore": 0.95}
  ],
  "confidence": 0.92,
  "groundedness": 0.95
}
```

## Step 1: Run the Example

```bash
cd examples/tutorials/rag-workflow
pip install -r requirements.txt
python rag_client.py
```

## Step 2: Understanding QoM for RAG

MPL's Quality of Meaning (QoM) metrics are particularly useful for RAG:

- **Schema Fidelity**: Ensures query and response match expected structure
- **Groundedness**: Measures how well the answer is supported by sources
- **Confidence**: Overall confidence in the generated answer

## What You'll See

```
Sending RAG query: What are the Q4 product launch features?
Response received:
  Answer: The Q4 launch includes three key features...
  Sources: 2 documents cited
  Confidence: 0.92
  Groundedness: 0.95
  Schema Fidelity: 1.0
```

## Next Steps

- Try the [Multi-Agent Tutorial](../multi-agent/README.md)
- Read about [QoM Profiles](../../../docs/qom-evaluation-engine.md)
