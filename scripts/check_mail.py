#!/usr/bin/env python3
"""
Sandy Mail Reader - Check ProtonMail via Hydroxide IMAP
"""
import imaplib
import email
from email.header import decode_header
import sys
import json

IMAP_HOST = '127.0.0.1'
IMAP_PORT = 1143
USERNAME = 'jens@protonmail.com'  # Change if different
PASSWORD = '+4BtFSZURGaa6BmbjNabDyto47b2coWwXA+dxeXBiUU='

def decode_str(s):
    """Decode email header string"""
    if isinstance(s, bytes):
        s = s.decode()
    decoded = decode_header(s)
    result = ''
    for text, encoding in decoded:
        if isinstance(text, bytes):
            text = text.decode(encoding or 'utf-8', errors='ignore')
        result += text
    return result

def check_mail(mailbox='INBOX', limit=10, unread_only=False):
    """Check mail from ProtonMail via Hydroxide"""
    try:
        # Connect to IMAP
        mail = imaplib.IMAP4(IMAP_HOST, IMAP_PORT)
        mail.login(USERNAME, PASSWORD)
        mail.select(mailbox)
        
        # Search for messages
        search_criteria = 'UNSEEN' if unread_only else 'ALL'
        status, messages = mail.search(None, search_criteria)
        
        if status != 'OK':
            return {'error': 'Failed to search messages'}
        
        email_ids = messages[0].split()
        email_ids = email_ids[-limit:]  # Get last N emails
        
        emails = []
        for email_id in reversed(email_ids):
            status, msg_data = mail.fetch(email_id, '(RFC822)')
            if status != 'OK':
                continue
                
            msg = email.message_from_bytes(msg_data[0][1])
            
            subject = decode_str(msg.get('Subject', ''))
            from_ = decode_str(msg.get('From', ''))
            date = msg.get('Date', '')
            
            # Get body
            body = ''
            if msg.is_multipart():
                for part in msg.walk():
                    if part.get_content_type() == 'text/plain':
                        body = part.get_payload(decode=True).decode(errors='ignore')
                        break
            else:
                body = msg.get_payload(decode=True).decode(errors='ignore')
            
            emails.append({
                'id': email_id.decode(),
                'subject': subject,
                'from': from_,
                'date': date,
                'body': body[:500]  # First 500 chars
            })
        
        mail.close()
        mail.logout()
        
        return {'count': len(emails), 'emails': emails}
        
    except Exception as e:
        return {'error': str(e)}

if __name__ == '__main__':
    unread = '--unread' in sys.argv
    limit = 10
    for i, arg in enumerate(sys.argv):
        if arg == '--limit' and i + 1 < len(sys.argv):
            limit = int(sys.argv[i + 1])
    
    result = check_mail(limit=limit, unread_only=unread)
    print(json.dumps(result, indent=2, ensure_ascii=False))
