#!/usr/bin/env python3
"""
Journalistic Research Script
Performs web research using DuckDuckGo and saves structured results as JSON.

Usage:
    python3 run_research.py "<query>" <output_path> [search_depth]
    
Arguments:
    query         - The search query (enclose in quotes if contains spaces)
    output_path   - Path where results JSON will be saved
    search_depth  - (Optional) "basic" or "advanced". Default: "basic"

Example:
    python3 run_research.py "AI news" storage/tasks/job_001/raw_data.json
    python3 run_research.py "climate solutions" storage/tasks/job_002/research.json advanced
"""

import sys
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import urlparse


def run_duckduckgo_search(query: str) -> list:
    """
    Run DuckDuckGo search using ddgr CLI tool.
    Falls back to simulated results if ddgr not available.
    """
    try:
        # Try to use ddgr (DuckDuckGo CLI)
        result = subprocess.run(
            ["ddgr", "--json", "--num", "10", query],
            capture_output=True,
            text=True,
            timeout=30
        )
        
        if result.returncode == 0:
            try:
                return json.loads(result.stdout)
            except json.JSONDecodeError:
                pass
    except (subprocess.CalledProcessError, FileNotFoundError):
        pass
    
    # Fallback: try to use curl with DuckDuckGo HTML scraping
    try:
        import requests
        from bs4 import BeautifulSoup
        
        url = f"https://html.duckduckgo.com/html/?q={query.replace(' ', '+')}"
        headers = {
            "User-Agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36"
        }
        
        response = requests.get(url, headers=headers, timeout=15)
        response.raise_for_status()
        
        soup = BeautifulSoup(response.text, 'html.parser')
        results = []
        
        for result in soup.select('.result'):
            title_elem = result.select_one('.result__a')
            snippet_elem = result.select_one('.result__snippet')
            
            if title_elem and snippet_elem:
                href = title_elem.get('href', '')
                if href.startswith('//'):
                    href = 'https:' + href
                elif href.startswith('/'):
                    continue
                    
                results.append({
                    'title': title_elem.get_text(strip=True),
                    'url': href,
                    'snippet': snippet_elem.get_text(strip=True)
                })
        
        return results[:10]
    except Exception:
        pass
    
    # Last resort: return empty (will trigger error)
    return []


def format_search_results(query: str, raw_results: list, search_depth: str) -> dict:
    """Format raw search results into structured JSON."""
    formatted = []
    sources = set()
    
    for item in raw_results:
        url = item.get('url', '')
        if not url:
            continue
            
        domain = urlparse(url).netloc.replace('www.', '')
        sources.add(domain)
        
        formatted.append({
            'title': item.get('title', 'Untitled'),
            'url': url,
            'snippet': item.get('snippet', item.get('abstract', 'No description available')),
            'source': domain
        })
    
    # Generate summary
    if formatted:
        summary = f"Found {len(formatted)} results for '{query}'. Top sources: {', '.join(list(sources)[:3])}."
    else:
        summary = f"No results found for '{query}'."
    
    return {
        'query': query,
        'timestamp': datetime.now(timezone.utc).isoformat(),
        'search_depth': search_depth,
        'results': formatted,
        'sources': list(sources),
        'summary': summary
    }


def perform_deep_search(query: str) -> list:
    """Perform multiple searches with variations for deeper results."""
    all_results = []
    
    # Original query
    all_results.extend(run_duckduckgo_search(query))
    
    # Variations for depth
    variations = [
        f"{query} latest news",
        f"{query} 2026",
        f"{query} analysis"
    ]
    
    for var in variations[:2]:  # Limit to 2 variations
        try:
            results = run_duckduckgo_search(var)
            # Filter duplicates by URL
            existing_urls = {r.get('url', '') for r in all_results}
            for r in results:
                if r.get('url') and r.get('url') not in existing_urls:
                    all_results.append(r)
        except Exception:
            continue
    
    return all_results


def main():
    if len(sys.argv) < 3:
        error_msg = "ERROR: Insufficient arguments\nUsage: python3 run_research.py '<query>' <output_path> [search_depth]"
        print(error_msg, file=sys.stderr)
        sys.exit(1)
    
    query = sys.argv[1]
    output_path = sys.argv[2]
    search_depth = sys.argv[3] if len(sys.argv) > 3 else "basic"
    
    # Validate search_depth
    if search_depth not in ["basic", "advanced"]:
        search_depth = "basic"
    
    try:
        # Ensure output directory exists
        output_file = Path(output_path)
        output_file.parent.mkdir(parents=True, exist_ok=True)
        
        # Perform search
        print(f"🔍 Researching: '{query}' ({search_depth} mode)", file=sys.stderr)
        
        if search_depth == "advanced":
            raw_results = perform_deep_search(query)
        else:
            raw_results = run_duckduckgo_search(query)
        
        # Check if we got results
        if not raw_results:
            error_output = {
                'query': query,
                'timestamp': datetime.now(timezone.utc).isoformat(),
                'search_depth': search_depth,
                'error': 'No search results found. The search service may be unavailable.',
                'results': [],
                'sources': [],
                'summary': f'Search failed for query: {query}'
            }
            output_file.write_text(json.dumps(error_output, indent=2))
            print(f"ERROR: No results found for '{query}'", file=sys.stderr)
            sys.exit(1)
        
        # Format results
        structured_data = format_search_results(query, raw_results, search_depth)
        
        # Write to file
        output_file.write_text(json.dumps(structured_data, indent=2))
        
        # Success output
        print(f"SUCCESS: Research data saved to {output_path}")
        print(f"  Found: {len(structured_data['results'])} results from {len(structured_data['sources'])} sources", file=sys.stderr)
        
    except Exception as e:
        error_output = {
            'query': query,
            'timestamp': datetime.now(timezone.utc).isoformat(),
            'search_depth': search_depth,
            'error': str(e),
            'results': [],
            'sources': [],
            'summary': f'Error during research: {str(e)}'
        }
        try:
            Path(output_path).write_text(json.dumps(error_output, indent=2))
        except:
            pass
        print(f"ERROR: {str(e)}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
