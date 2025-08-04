module Jekyll
  module FootnoteSidenote
    Jekyll::Hooks.register [:posts, :pages], :pre_render do |doc|
      # Only process Markdown files
      next unless doc.extname =~ /^\.md$/i

    #   puts "Sidenote: Processing #{doc.path}"

      # Step 1: Collect footnote definitions
      footnotes = {}
      doc.content.scan(/^\s*\[\^(.+)\]:\s*(.+)$/) do |id, text|
        # puts "Hi: #{id}"
        footnotes[id] = text.strip
      end
    #   puts "Found #{footnotes.size} footnotes"

      # Step 2: Remove footnote definitions
      doc.content = doc.content.gsub(/^\s*\[\^(.+)\]:\s*.+$/, '')

      # Step 3: Initialize counter
      counter = 0

      # Step 4: Replace footnote references with HTML template
      doc.content = doc.content.gsub(/\[\^(.+)\]/) do |match|
        ref_id = Regexp.last_match(1)
        footnote_text = footnotes[ref_id] || ''
        counter += 1
        unique_id = "rf#{counter}"

        # puts "Replacing [#{match}] with HTML for footnote: #{footnote_text} (counter: #{counter})"

        # HTML template with separate counter
        "<input type=\"checkbox\" id=\"#{unique_id}\" class=\"sidenote\"/><label for=\"#{unique_id}\"><sup>#{counter}</sup></label><span><sup>#{counter}</sup> #{footnote_text}</span>"
      end
    end
  end
end
