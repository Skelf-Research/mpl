#!/usr/bin/env python3
"""
RAG Workflow Tutorial

Demonstrates typed RAG queries with MPL validation and QoM metrics.
"""

import json
import requests
from typing import Optional

# MPL Proxy URL
PROXY_URL = "http://localhost:9443"


def send_rag_query(
    query: str,
    max_documents: int = 5,
    min_relevance: float = 0.7,
    sources: Optional[list] = None
) -> dict:
    """Send a RAG query through the MPL proxy."""
    payload = {
        "queryId": f"q-{hash(query) % 10000:04d}",
        "query": query,
        "context": {
            "maxDocuments": max_documents,
            "minRelevanceScore": min_relevance,
        },
        "responseFormat": "markdown"
    }

    if sources:
        payload["context"]["sources"] = sources

    print(f"\nSending RAG query: {query[:50]}...")

    response = requests.post(
        f"{PROXY_URL}/api/rag/query",
        json=payload,
        headers={
            "Content-Type": "application/json",
            "X-MPL-SType": "eval.rag.RAGQuery.v1",
        }
    )

    # Check MPL headers
    print(f"Status: {response.status_code}")
    print(f"Schema Fidelity: {response.headers.get('X-MPL-Schema-Fidelity', 'N/A')}")

    return response.json()


def parse_rag_response(response: dict) -> None:
    """Parse and display a RAG response."""
    print("\nRAG Response:")
    print("-" * 40)

    if "answer" in response:
        print(f"Answer: {response['answer'][:200]}...")

    if "sources" in response:
        print(f"\nSources ({len(response['sources'])} documents):")
        for i, source in enumerate(response["sources"][:3], 1):
            print(f"  {i}. {source.get('title', source.get('documentId', 'Unknown'))}")
            print(f"     Relevance: {source.get('relevanceScore', 'N/A')}")

    if "confidence" in response:
        print(f"\nConfidence: {response['confidence']:.2f}")

    if "groundedness" in response:
        print(f"Groundedness: {response['groundedness']:.2f}")


def main():
    print("=" * 50)
    print("RAG Workflow Tutorial")
    print("=" * 50)

    # Example 1: Basic RAG query
    print("\n1. Basic RAG Query")
    response = send_rag_query(
        query="What are the key features of the Q4 product launch?",
        max_documents=5,
        sources=["product-docs", "announcements"]
    )
    parse_rag_response(response)

    # Example 2: Query with high relevance threshold
    print("\n2. High-Precision Query")
    response = send_rag_query(
        query="What are the exact pricing tiers for enterprise customers?",
        max_documents=3,
        min_relevance=0.9,
        sources=["pricing-docs"]
    )
    parse_rag_response(response)

    # Example 3: Query with time constraints
    print("\n3. Time-Bounded Query")
    payload = {
        "queryId": "q-time-001",
        "query": "What product updates were announced last quarter?",
        "context": {
            "maxDocuments": 10,
            "minRelevanceScore": 0.6,
            "timeRange": {
                "start": "2024-07-01T00:00:00Z",
                "end": "2024-09-30T23:59:59Z"
            }
        }
    }

    response = requests.post(
        f"{PROXY_URL}/api/rag/query",
        json=payload,
        headers={
            "Content-Type": "application/json",
            "X-MPL-SType": "eval.rag.RAGQuery.v1",
        }
    )
    print(f"\nStatus: {response.status_code}")
    print(f"Schema Fidelity: {response.headers.get('X-MPL-Schema-Fidelity', 'N/A')}")
    parse_rag_response(response.json())

    # Example 4: Invalid query (missing required field)
    print("\n4. Invalid Query (missing 'query' field)")
    response = requests.post(
        f"{PROXY_URL}/api/rag/query",
        json={
            "queryId": "q-invalid",
            "context": {"maxDocuments": 5}
            # Missing required "query" field
        },
        headers={
            "Content-Type": "application/json",
            "X-MPL-SType": "eval.rag.RAGQuery.v1",
        }
    )
    print(f"Status: {response.status_code}")
    print(f"Schema Fidelity: {response.headers.get('X-MPL-Schema-Fidelity', 'N/A')}")
    if response.headers.get("X-MPL-Validation-Error"):
        print(f"Validation Error: {response.headers['X-MPL-Validation-Error']}")

    print("\n" + "=" * 50)
    print("Tutorial complete!")
    print("=" * 50)


if __name__ == "__main__":
    main()
