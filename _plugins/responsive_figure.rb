Jekyll::Hooks.register [:posts, :pages], :pre_render do |doc|
    # match ![alt](/assets/src)
    #       *caption*
    #
    # This content will eagerly match any caption under the image, even if their indent does not match.
    # It works well as long as any non-caption content is not directly put under the image and starts with *.
    doc.content = doc.content.gsub(/(?mi)^(\s*)!\[([^\]]*)\]\(\/assets\/([^)]+\.(?:jpg|jpeg|png|tiff|gif))\)(?:\r?\n\s*(?:\*([^\*]+)\*))?/) do
      indent   = Regexp.last_match(1)
      alt_text = Regexp.last_match(2)
      src      = Regexp.last_match(3)
      caption  = Regexp.last_match(4)

      replacement = "{% include figure.liquid alt=\"#{alt_text}\" path=\"/assets/#{src}\" indent=\"#{indent}\" caption=\"#{caption}\" %}"
    end
  end
