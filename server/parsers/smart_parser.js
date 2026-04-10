const cheerio = require('cheerio');

//  intelligent web content parser
function smartParseHTML(html, url, customSelector = null) {
  const $ = cheerio.load(html);
  const newsItems = [];
  
  console.log('Starting smart parsing...');
  
  // If custom selector is provided, try it first
  if (customSelector && customSelector.trim()) {
    console.log('Trying custom selector:', customSelector);
    const elements = $(customSelector);
    console.log('Found elements with custom selector:', elements.length);
    
    if (elements.length > 0) {
      elements.each((index, element) => {
        if (index >= 20) return false;
        
        const item = extractNewsItem($, $(element), url, index);
        if (item && item.title && item.title.length > 0) {
          newsItems.push(item);
        }
      });
      
      if (newsItems.length > 0) {
        console.log(`Successfully extracted ${newsItems.length} items with custom selector`);
        return newsItems;
      }
    }
  }
  
  // Smart detection strategies
  const strategies = [
    // Strategy 1: Look for list items with links
    () => {
      console.log('Strategy 1: Looking for list items with links...');
      const items = [];
      
      // Try different list patterns
      const listSelectors = [
        '.news-list li',
        '.content-list li',
        'ul.news-list li',
        'ol.news-list li',
        'article',
        '.post',
        '.item',
        '.card'
      ];
      
      for (const selector of listSelectors) {
        const elements = $(selector);
        if (elements.length > 0) {
          console.log(`Found ${elements.length} elements with ${selector}`);
          
          elements.each((index, element) => {
            if (index >= 20) return false;
            
            const $element = $(element);
            const item = extractNewsItem($, $element, url, index);
            if (item && item.title && item.title.length > 10 && item.title.length < 200) {
              // Filter out navigation menu items
              if (!item.title.match(/^(home|about|contact|login|register|search|menu|nav|more|click|view|macro|company|fund|broker)/i)) {
                items.push(item);
              }
            }
          });
          
          if (items.length >= 3) {
            console.log(`Strategy 1 success: extracted ${items.length} items`);
            return items;
          }
        }
      }
      
      // If no specific news lists found, try generic lists but with better filtering
      console.log('Trying generic lists with better filtering...');
      const genericSelectors = ['ul li', 'ol li'];
      
      for (const selector of genericSelectors) {
        const elements = $(selector);
        if (elements.length > 0) {
          console.log(`Found ${elements.length} elements with ${selector}`);
          
          elements.each((index, element) => {
            if (index >= 20) return false;
            
            const $element = $(element);
            const item = extractNewsItem($, $element, url, index);
            
            // More strict filtering for generic lists
            if (item && item.title && 
                item.title.length > 15 && 
                item.title.length < 200 &&
                !item.title.match(/^(home|about|contact|login|register|search|menu|nav|more|click|view|macro|company|fund|broker|english|today|paper)/i) &&
                item.url.includes('articles')) { // Must be article links
              items.push(item);
            }
          });
          
          if (items.length >= 3) {
            console.log(`Strategy 1 success: extracted ${items.length} items`);
            return items;
          }
        }
      }
      
      return null;
    },
    
    // Strategy 2: Look for headings with links
    () => {
      console.log('Strategy 2: Looking for headings with links...');
      const items = [];
      
      // Try headings with links and also headings that contain links
      $('h1 a, h2 a, h3 a, h4 a, h5 a, h6 a').each((index, element) => {
        if (index >= 20) return false;
        
        const $element = $(element);
        const title = $element.text().trim();
        const href = $element.attr('href');
        
        if (title && title.length > 5 && title.length < 200 && href) {
          items.push({
            id: require('uuid').v4(),
            title: title,
            url: href.startsWith('http') ? href : new URL(href, url).href,
            desc: `From ${url}`,
            cover: null,
            author: null,
            timestamp: new Date().toISOString(),
            hot: null,
            mobile_url: null
          });
        }
      });
      
      // Also try headings that contain links inside
      if (items.length < 5) {
        $('h1, h2, h3, h4, h5, h6').each((index, element) => {
          if (index >= 20) return false;
          
          const $element = $(element);
          const title = $element.text().trim();
          const href = $element.find('a').attr('href');
          
          if (title && title.length > 10 && title.length < 200 && href && 
              !title.match(/^(home|about|contact|login|register|search|menu|nav|more)/i)) {
            items.push({
              id: require('uuid').v4(),
              title: title,
              url: href.startsWith('http') ? href : new URL(href, url).href,
              desc: `From ${url}`,
              cover: null,
              author: null,
              timestamp: new Date().toISOString(),
              hot: null,
              mobile_url: null
            });
          }
        });
      }
      
      if (items.length > 0) {
        console.log(`Strategy 2 success: extracted ${items.length} items`);
        return items;
      }
      
      return null;
    },
    
    // Strategy 3: Look for news content patterns
    () => {
      console.log('Strategy 3: Looking for news content patterns...');
      const items = [];
      
      // Try to find elements that look like news items
      const newsPatterns = [
        // Elements with both title and time
        'li:has(a):has(.time)',
        'li:has(a):has(span)',
        '.news-item',
        '.article',
        '.post',
        // Links with substantial content
        'a[href*="/article"]',
        'a[href*="/news"]',
        'a[href*="/post"]'
      ];
      
      for (const pattern of newsPatterns) {
        const elements = $(pattern);
        if (elements.length > 0) {
          console.log(`Found ${elements.length} elements with pattern: ${pattern}`);
          
          elements.each((index, element) => {
            if (index >= 20) return false;
            
            const $element = $(element);
            const item = extractNewsItem($, $element, url, index);
            if (item && item.title && item.title.length > 10 && item.title.length < 200) {
              // Filter out navigation
              if (!item.title.match(/^(home|about|contact|login|register|search|menu|nav|more|click|view)/i)) {
                items.push(item);
              }
            }
          });
          
          if (items.length >= 5) {
            break; // Found enough items
          }
        }
      }
      
      if (items.length > 0) {
        console.log(`Strategy 3 success: extracted ${items.length} items`);
        return items;
      }
      
      return null;
    },
    
    // Strategy 4: Look for any links with substantial text
    () => {
      console.log('Strategy 4: Looking for any links with substantial text...');
      const items = [];
      
      $('a[href]').each((index, element) => {
        if (index >= 20) return false;
        
        const $element = $(element);
        const title = $element.text().trim();
        const href = $element.attr('href');
        
        // Filter out navigation links
        if (title && title.length > 15 && title.length < 200 && 
            href && !href.includes('#') && 
            !title.match(/^(home|about|contact|login|register|search|menu|nav|more|click|view)/i)) {
          items.push({
            id: require('uuid').v4(),
            title: title,
            url: href.startsWith('http') ? href : new URL(href, url).href,
            desc: `From ${url}`,
            cover: null,
            author: null,
            timestamp: new Date().toISOString(),
            hot: null,
            mobile_url: null
          });
        }
      });
      
      if (items.length > 0) {
        console.log(`Strategy 4 success: extracted ${items.length} items`);
        return items;
      }
      
      return null;
    }
  ];
  
  // Try each strategy
  for (const strategy of strategies) {
    const result = strategy();
    if (result && result.length > 0) {
      return result;
    }
  }
  
  console.log('All strategies failed, no items extracted');
  return [];
}

// Helper function to extract news item from element
function extractNewsItem($, $element, baseUrl, index) {
  // Try to find title
  let title = '';
  
  // First try direct text
  title = $element.text().trim();
  
  // If it's a link element, use its text
  if ($element.is('a')) {
    title = $element.text().trim();
  }
  // If it contains a link, use the link's text
  else if ($element.find('a').length > 0) {
    title = $element.find('a').first().text().trim();
  }
  // Try common title selectors within the element
  else {
    const titleSelectors = ['h1', 'h2', 'h3', 'h4', 'h5', 'h6', '.title', '.headline', '.name', 'p'];
    for (const selector of titleSelectors) {
      const $title = $element.find(selector).first();
      if ($title.length > 0) {
        title = $title.text().trim();
        break;
      }
    }
  }
  
  // Clean up title
  title = title.replace(/\s+/g, ' ').trim();
  
  // Try to find URL
  let url = '';
  
  // If element is a link
  if ($element.is('a')) {
    url = $element.attr('href');
  }
  // If element contains a link
  else if ($element.find('a').length > 0) {
    url = $element.find('a').first().attr('href');
  }
  // If element has data-href or similar attributes
  else {
    url = $element.attr('href') || $element.attr('data-url') || $element.attr('data-link');
  }
  
  // Make URL absolute
  if (url) {
    url = url.startsWith('http') ? url : new URL(url, baseUrl).href;
  } else {
    url = baseUrl;
  }
  
  // Try to find description
  let desc = '';
  const descSelectors = ['.desc', '.description', '.summary', '.excerpt', 'p'];
  for (const selector of descSelectors) {
    const $desc = $element.find(selector).first();
    if ($desc.length > 0) {
      desc = $desc.text().trim().replace(/\s+/g, ' ');
      if (desc.length > 10 && desc !== title) {
        break;
      }
    }
  }
  
  // Try to find timestamp
  let timestamp = new Date().toISOString();
  const timeSelectors = ['.time', '.date', '.timestamp', 'time', '.pub-date'];
  for (const selector of timeSelectors) {
    const $time = $element.find(selector).first();
    if ($time.length > 0) {
      const timeText = $time.text().trim();
      const parsedTime = parseTimeText(timeText);
      if (parsedTime) {
        timestamp = parsedTime;
        break;
      }
    }
  }
  
  // Validate and return
  if (title && title.length > 3 && title.length < 300) {
    return {
      id: require('uuid').v4(),
      title: title,
      url: url,
      desc: desc || `From ${baseUrl}`,
      cover: null,
      author: null,
      timestamp: timestamp,
      hot: null,
      mobile_url: null
    };
  }
  
  return null;
}

// Helper function to parse time text
function parseTimeText(timeText) {
  // Simple time parsing - can be enhanced
  const now = new Date();
  
  // Handle relative times
  if (timeText.includes('ago') || timeText.includes('before')) {
    return now.toISOString();
  }
  
  // Handle common date formats
  const dateMatch = timeText.match(/(\d{4})-(\d{1,2})-(\d{1,2})/);
  if (dateMatch) {
    const date = new Date(dateMatch[1], dateMatch[2] - 1, dateMatch[3]);
    return date.toISOString();
  }
  
  return now.toISOString();
}

module.exports = { smartParseHTML };
