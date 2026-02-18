#!/usr/bin/env python3
"""
Journalistic Writing Script
Transforms research data into readable articles. Reads JSON research data and produces markdown.

Usage:
    python3 write_article.py <input_path> <output_path> [options]
    
Arguments:
    input_path    - Path to research JSON file (from run_research.py)
    output_path   - Path where article markdown will be saved
    --style       - (Optional) Writing style: "neutral", "analytical", "engaging". Default: "neutral"
    --length      - (Optional) Article length: "short", "medium", "long". Default: "medium"

Example:
    python3 write_article.py storage/tasks/job_001/raw_data.json storage/tasks/job_001/article.md
    python3 write_article.py data.json article.md --style analytical --length short
"""

import sys
import json
import argparse
from datetime import datetime, timezone
from pathlib import Path
from typing import Dict, List, Any


def load_research_data(input_path: str) -> Dict[str, Any]:
    """Load and validate research data from JSON file."""
    input_file = Path(input_path)
    
    if not input_file.exists():
        raise FileNotFoundError(f"Input file not found: {input_path}")
    
    try:
        with open(input_file, 'r') as f:
            data = json.load(f)
    except json.JSONDecodeError as e:
        raise ValueError(f"Invalid JSON in input file: {e}")
    
    # Check for error in research data
    if 'error' in data:
        raise ValueError(f"Research data contains error: {data['error']}")
    
    # Validate required fields
    if 'results' not in data:
        raise ValueError("Research data missing 'results' field")
    
    return data


def extract_title(query: str) -> str:
    """Generate article title from research query."""
    # Capitalize first letter of each word
    words = query.split()
    title_words = []
    
    for word in words:
        # Skip common articles and prepositions unless first word
        if not title_words and word.lower() in ['the', 'a', 'an']:
            title_words.append(word.capitalize())
        elif word.lower() in ['the', 'a', 'an', 'in', 'on', 'at', 'to', 'for', 'of', 'with', 'by']:
            title_words.append(word.lower())
        else:
            title_words.append(word.capitalize())
    
    return ' '.join(title_words)


def format_sources(results: List[Dict[str, Any]]) -> str:
    """Format source list for article."""
    sources_md = "## Sources\n\n"
    
    for result in results:
        title = result.get('title', 'Untitled')
        url = result.get('url', '')
        source = result.get('source', 'Unknown')
        
        if url:
            sources_md += f"- [{title}]({url}) - {source}\n"
        else:
            sources_md += f"- {title} - {source}\n"
    
    return sources_md


def generate_summary(results: List[Dict[str, Any]], query: str) -> str:
    """Generate article summary from results."""
    if not results:
        return f"No specific information found for '{query}'."
    
    # Extract key themes from snippets
    snippets = [r.get('snippet', '') for r in results if r.get('snippet')]
    
    if not snippets:
        return f"Research on '{query}' returned results but detailed content was limited."
    
    # Simple summary based on first few results
    sources = list(set([r.get('source', 'Unknown') for r in results[:5]]))
    
    summary = f"Recent research on '{query}' reveals information from multiple sources including {', '.join(sources[:3])}. "
    summary += f"The findings span {len(results)} distinct sources and cover various aspects of the topic."
    
    return summary


def format_content(results: List[Dict[str, Any]], style: str) -> str:
    """Format main content based on writing style."""
    if not results:
        return "## Content\n\nNo detailed content available from research sources.\n"
    
    content_md = "## Key Findings\n\n"
    
    # Group results by source for variety
    sources_content = {}
    for result in results:
        source = result.get('source', 'Unknown')
        if source not in sources_content:
            sources_content[source] = []
        sources_content[source].append(result)
    
    # Generate paragraphs
    if style == "engaging":
        content_md += "Recent developments have brought new attention to this topic. "
        content_md += "Here's what the research reveals:\n\n"
        
        for i, result in enumerate(results[:5], 1):
            snippet = result.get('snippet', '')
            title = result.get('title', 'Untitled')
            if snippet:
                content_md += f"**{i}. {title}**\n\n{snippet}\n\n"
    
    elif style == "analytical":
        content_md += "Analysis of the available data reveals several important patterns:\n\n"
        
        for result in results[:5]:
            title = result.get('title', 'Untitled')
            snippet = result.get('snippet', '')
            source = result.get('source', 'Unknown')
            
            if snippet:
                content_md += f"- **{title}** ({source}): {snippet}\n\n"
    
    else:  # neutral
        content_md += "The following information was gathered from research:\n\n"
        
        for result in results[:7]:
            title = result.get('title', 'Untitled')
            snippet = result.get('snippet', '')
            
            if snippet:
                content_md += f"### {title}\n\n{snippet}\n\n"
    
    return content_md


def adjust_length(content: str, length: str, results: List[Dict[str, Any]]) -> str:
    """Adjust content length based on parameter."""
    if length == "short":
        # Keep only first 3 results
        return content[:2000] if len(content) > 2000 else content
    
    elif length == "long":
        # Add additional context section
        additional = "\n\n## Additional Context\n\n"
        additional += f"Research conducted on {datetime.now(timezone.utc).strftime('%B %d, %Y')}. "
        additional += f"Data sourced from {len(results)} unique sources. "
        additional += "This article presents a comprehensive overview based on available information."
        return content + additional
    
    else:  # medium - default
        return content


def generate_article(data: Dict[str, Any], style: str, length: str) -> str:
    """Generate complete markdown article from research data."""
    query = data.get('query', 'Research Topic')
    results = data.get('results', [])
    research_timestamp = data.get('timestamp', datetime.now(timezone.utc).isoformat())
    
    # Build article
    article = []
    
    # Title
    title = extract_title(query)
    article.append(f"# {title}\n")
    
    # Metadata
    article.append(f"*Research Date: {research_timestamp}*\n")
    article.append(f"*Sources: {len(results)}*\n")
    
    # Summary
    article.append("## Summary\n")
    article.append(generate_summary(results, query))
    article.append("")
    
    # Main content
    article.append(format_content(results, style))
    
    # Sources
    if results:
        article.append(format_sources(results))
    
    # Join and adjust length
    full_article = '\n'.join(article)
    return adjust_length(full_article, length, results)


def main():
    # Parse arguments
    parser = argparse.ArgumentParser(
        description='Transform research data into journalistic articles',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog='''
Examples:
    python3 write_article.py input.json output.md
    python3 write_article.py data.json article.md --style engaging --length short
    python3 write_article.py research.json output.md --style analytical
        '''
    )
    
    parser.add_argument('input_path', help='Path to research JSON file')
    parser.add_argument('output_path', help='Path where article will be saved')
    parser.add_argument('--style', choices=['neutral', 'analytical', 'engaging'],
                       default='neutral', help='Writing style (default: neutral)')
    parser.add_argument('--length', choices=['short', 'medium', 'long'],
                       default='medium', help='Article length (default: medium)')
    
    args = parser.parse_args()
    
    try:
        # Load research data
        print(f"📖 Reading research data from: {args.input_path}", file=sys.stderr)
        data = load_research_data(args.input_path)
        
        # Generate article
        print(f"✍️  Writing article in {args.style} style ({args.length} length)", file=sys.stderr)
        article = generate_article(data, args.style, args.length)
        
        # Ensure output directory exists
        output_file = Path(args.output_path)
        output_file.parent.mkdir(parents=True, exist_ok=True)
        
        # Write article
        output_file.write_text(article)
        
        # Success
        word_count = len(article.split())
        print(f"SUCCESS: Article saved to {args.output_path}")
        print(f"  Word count: {word_count} words", file=sys.stderr)
        print(f"  Style: {args.style}, Length: {args.length}", file=sys.stderr)
        
    except FileNotFoundError as e:
        error_msg = f"ERROR: {e}"
        print(error_msg, file=sys.stderr)
        
        # Write error to output file
        try:
            Path(args.output_path).write_text(f"# Error\n\n{error_msg}\n")
        except:
            pass
        sys.exit(1)
        
    except ValueError as e:
        error_msg = f"ERROR: {e}"
        print(error_msg, file=sys.stderr)
        
        try:
            Path(args.output_path).write_text(f"# Error\n\n{error_msg}\n")
        except:
            pass
        sys.exit(1)
        
    except Exception as e:
        error_msg = f"ERROR: Unexpected error: {e}"
        print(error_msg, file=sys.stderr)
        
        try:
            Path(args.output_path).write_text(f"# Error\n\n{error_msg}\n")
        except:
            pass
        sys.exit(1)


if __name__ == "__main__":
    main()
